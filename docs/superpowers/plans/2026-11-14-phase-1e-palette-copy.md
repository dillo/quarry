# Phase 1E: The ⌘K Palette and Copying Results — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish Phase 1 with the ⌘K command palette (open tabs, recently closed tabs, tables and views, commands) and copying results as TSV, CSV or JSON. Also close the four leftovers from 1D: saves that could land out of order, two copies of the reply poll, a `settle()` that hid timeouts, and view tests that were never written.

**Architecture:**
- **`quarry-engine`:** the `Exporter` trait and `ExporterRegistry` (TSV, CSV, JSON).
- **`quarry-store`:** schema v2 with a `closed_tabs` table, and `Drafts::closed`.
- **`app` models:** GPUI-free and mutation-tested.
  - `palette`: filtering and ranking.
  - `copy`: what a copy takes, and its note.
  - `closed_tabs`: the closed-tab list.
  - `save_order`: save generations.
- **`app` views:**
  - `PaletteSource` and `PaletteSourceRegistry`, one module per source.
  - A palette dialog over gpui-component's `Command`.
  - `QueryTab::copy`, and the workspace's status-bar note.
  - Saves through one lock.
  - `poll_reply`.
- **`app` shell:** one command table that makes the menus, feeds the bindings test and lists the palette's commands.
- **Status:** every task's code was built and gated in a scratch clone first, one commit per task. The tasks were then replayed in order on a fresh branch from the spec commit, and each commit passed fmt, clippy, the quality checks, typos, complexity and the whole workspace test suite. The suite grew from 318 tests after Task 1 to 382 after Task 8. `just check` passed at the end. Each task's code below is taken from those commits.

**Tech Stack:** Rust 1.98.1, `gpui-kit` =0.9.0 (gpui-component's `Command`, `Dialog` and `Root`), `rusqlite` 0.41.1, `async-channel`.

**Spec:** `docs/superpowers/specs/2026-11-13-phase-1e-palette-copy-design.md` (approved 2026-11-13). Parent: `docs/superpowers/specs/2026-11-04-quarry-design.md` §3.3, §4.2 and §9. ADR 0010 (updated in Task 5) and ADR 0011 (written in Task 9).

## Global Constraints

- **Branch and commits:** work on `phase-1e-palette-copy`, which already holds the spec and this plan. Use Conventional Commits, with no trailers.
- **No completion of any kind:**
  - The editor gets no completion, hover, definition or code-action providers, no inline suggestions and no hints. The existing test keeps asserting it.
  - The palette navigates and runs commands; it never offers SQL.
- **Extension points:** new kinds of things go through traits and registries: `Exporter`/`ExporterRegistry` and `PaletteSource`/`PaletteSourceRegistry`. No `match` on a registry-owned enum outside its module.
- **Dependency direction:**
  - `engine` depends on no workspace crate.
  - `ui-kit` depends only on GPUI.
  - `store` may depend on `engine` but needs no workspace crate in 1E.
  - `cargo xtask check-deps` enforces all three.
- **Coverage:**
  - `engine` and `store` need at least 95% lines and 90% regions.
  - `app` needs at least 80% lines and 70% regions.
  - Every file in `crates/app/src/model` stays at 100% lines.
- **Mutation testing:** 0 missed in `engine`, `store`, `crates/app/src/model` and `crates/app/src/sample`.
- **Per function:** at most 15 statements and 40 lines, cyclomatic ≤ 10, cognitive ≤ 15, ≤ 5 arguments and nesting ≤ 4. This applies to tests too.
- **Banned outside `#[test]` functions:** `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` and `dbg!`.
  - Test helpers that aren't `#[test]` functions count as outside.
  - No lint suppression attributes, and no `as` casts.
- **Polling:** GPUI code never awaits a session channel. It polls every `POLL` (33 ms). Background work uses GPUI's executor, so tests stay deterministic.
- **Contrast:** every new color goes through a `Role` and is covered by `tests/contrast.rs`. 1E adds no new color; the palette uses `Role::TextMuted` and the existing component theme.
- **Exact versions:** `gpui-kit` =0.9.0, `rusqlite` 0.41.1 (the workspace dependency).
- **Headless testing:**
  - Tests find elements by id, so elements they look for call `.test_support()`.
  - `settle()` advances the clock by `POLL`, pumps the executor, and returns whether its condition held. Every caller asserts it.
- **Every task ends with its gates passing.** Don't commit anything under `.superpowers/`.
- **Merging:** only with the user's explicit go-ahead, after the user's visual review.

## Where the plan departs from the spec

The prototype found these. Each is small, and all of them are in the code below.

1. **`SaveOrder::issue`, not `next`.** Clippy's `should_implement_trait` rejects a method named `next` that isn't `Iterator::next`.
2. **`model::copy::Copied`, not `Copy`.** A type named `Copy` would shadow the prelude's trait. `Copied::Rows` also carries `of`, the number of rows the copy stands for, which the "Copied 100,000 of 250,000 rows" note needs.
3. **`QueryTab::copy(format, cx)` takes an exporter id** (`"tsv"`, `"csv"` or `"json"`), not a `&dyn Exporter`. Formatting runs on the background executor, which needs `'static` data, so the task looks the exporter up there.
4. **A third ⌘K binding, in the palette's own `Palette` key context.** The dialog renders in gpui-kit's `Root`, outside the `Workspace` context, so this binding is what lets ⌘K close an open palette.
5. **The command table is `Vec<Command>` with `Command { label, action: fn() -> Box<dyn Action>, menu }`.** The palette leaves out `APP_MENU` ("Quarry"), which holds only Quit.
6. **A closed tab's id isn't stored.** `closed_tabs` has no id column (as the spec says), so a closed draft loads with id 0, and reopening it gives it a new id.
7. **Small additions the behavior needs:**
   - `TabList::reopen(title)`.
   - `schema_tree::row_of` and `schema_tree::summary`.
   - `closed_tabs::first_line`.
   - `SidebarItem::track_focus` in ui-kit.
   - `SchemaTree` now scrolls itself, so the palette can scroll a table into view.
8. **Test windows sit in gpui-kit's `Root`,** as the real window does, so dialogs can open in tests.

## File Structure

```
crates/engine/src/export.rs, export/{tsv,csv,json}.rs     Exporter, ExporterRegistry, formats
crates/store/src/{drafts.rs, migrations.rs, sqlite.rs}    Drafts::closed, schema v2
crates/app/src/model/
  palette.rs        PaletteItem, filter, PALETTE_LIMIT
  copy.rs           Selection, Copied, copied(), note()
  closed_tabs.rs    remember, take, first_line, CLOSED_LIMIT
  save_order.rs     SaveOrder
  tabs.rs, schema_tree.rs                                 (changed)
crates/app/src/views/
  poll.rs           poll_reply
  palette/{mod.rs, dialog.rs, tabs.rs, closed.rs, tables.rs, commands.rs}
  query_tab/copy.rs QueryTab::copy
  workspace/{copy.rs, palette.rs}, drafts.rs, tabs.rs, ...  (changed)
  schema.rs                                               (changed)
crates/app/src/shell/actions.rs                           command table, bindings, menus
crates/ui-kit/src/components/sidebar_item.rs              track_focus
crates/app/tests/app/{copy.rs, palette.rs}, ...           GPUI tests
docs/adr/0011-command-palette-and-copying.md
```

---

### Task 1: Engine: `Exporter`, `ExporterRegistry`, and TSV, CSV and JSON

The engine learns to turn rows into text. Each format is an `Exporter` in its own module, and `ExporterRegistry::standard()` lists TSV, CSV and JSON in that order. The exporters format full `Value`s, never the grid's cut-off cell text, and each module carries its own tests for `NULL`, numbers, non-finite floats, bytes, quoting, the header, no rows and single values.

**Files:**
- Create: `crates/engine/src/export.rs`
- Create: `crates/engine/src/export/csv.rs`
- Create: `crates/engine/src/export/json.rs`
- Create: `crates/engine/src/export/tsv.rs`
- Modify: `crates/engine/src/lib.rs`

**Interfaces:**
- Consumes: `quarry_engine::{result::{Column, Row}, value::Value}`.
- Produces: `quarry_engine::export::{Exporter: Send + Sync { id(&self) -> &'static str; title(&self) -> &'static str; rows(&self, &[Column], &[Row]) -> String; value(&self, &Value) -> String }, ExporterRegistry::{standard() -> Self, ids(&self) -> impl Iterator<Item = &'static str>, get(&self, &str) -> Option<&dyn Exporter>}}`. Ids are `"tsv"`, `"csv"`, `"json"`; titles `"TSV"`, `"CSV"`, `"JSON"`.

- [ ] **Step 1: Implement**

Each module's tests sit beside its code, as in the rest of the engine.

Create `crates/engine/src/export.rs`:
```rust
//! Turns result rows into text to copy: the exporter extension point.
//!
//! Each format is an [`Exporter`] in its own module; adding one means a new
//! module and one line in [`ExporterRegistry::standard`].

mod csv;
mod json;
mod tsv;

use std::fmt::Write;

use crate::{
    result::{Column, Row},
    value::Value,
};

/// One text format for rows and single values.
pub trait Exporter: Send + Sync {
    /// A stable name, such as "csv".
    fn id(&self) -> &'static str;

    /// The name people see, such as "CSV".
    fn title(&self) -> &'static str;

    /// `rows` under a header of `columns`, in this format.
    fn rows(&self, columns: &[Column], rows: &[Row]) -> String;

    /// One cell on its own, in this format.
    fn value(&self, value: &Value) -> String;
}

/// The formats Quarry copies to, in menu order.
pub struct ExporterRegistry {
    exporters: Vec<Box<dyn Exporter>>,
}

impl ExporterRegistry {
    /// TSV, CSV and JSON, in that order.
    pub fn standard() -> Self {
        Self {
            exporters: vec![Box::new(tsv::Tsv), Box::new(csv::Csv), Box::new(json::Json)],
        }
    }

    /// The registered ids, in order.
    pub fn ids(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.exporters.iter().map(|exporter| exporter.id())
    }

    /// The exporter named `id`.
    pub fn get(&self, id: &str) -> Option<&dyn Exporter> {
        self.exporters
            .iter()
            .find(|exporter| exporter.id() == id)
            .map(AsRef::as_ref)
    }
}

/// A value's full text, as the delimited formats write it: `NULL` is
/// empty, numbers are the engine's text, and bytes are `\x` and hex.
fn plain(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bytes(bytes) => hex(bytes),
        other => other.to_string(),
    }
}

/// `\x` followed by the bytes in lowercase hex, as in `\x00ff`.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::from("\\x"), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// `text` in double quotes, with each quote inside doubled.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

/// One line per row under a header line, each field made by `field`,
/// separated by `separator` and ended by `end`.
fn delimited(
    columns: &[Column],
    rows: &[Row],
    field: impl Fn(&str) -> String,
    separator: &str,
    end: &str,
) -> String {
    let line = |fields: Vec<String>| fields.join(separator) + end;
    let header = line(columns.iter().map(|column| field(&column.name)).collect());
    let body = rows
        .iter()
        .map(|row| line(row.iter().map(|value| field(&plain(value))).collect()));
    std::iter::once(header).chain(body).collect()
}

#[cfg(test)]
mod tests {
    use super::{ExporterRegistry, hex, plain, quoted};
    use crate::value::Value;

    #[test]
    fn the_standard_formats_are_tsv_csv_and_json() {
        let registry = ExporterRegistry::standard();
        assert_eq!(registry.ids().collect::<Vec<_>>(), ["tsv", "csv", "json"]);
        let titles: Vec<_> = registry
            .ids()
            .filter_map(|id| registry.get(id))
            .map(super::Exporter::title)
            .collect();
        assert_eq!(titles, ["TSV", "CSV", "JSON"]);
        assert!(registry.get("xml").is_none());
    }

    #[test]
    fn plain_text_is_the_full_value() {
        assert_eq!(plain(&Value::Null), "");
        assert_eq!(plain(&Value::Int(-3)), "-3");
        assert_eq!(plain(&Value::Float(2.5)), "2.5");
        assert_eq!(plain(&Value::Text("x".repeat(300))), "x".repeat(300));
        assert_eq!(plain(&Value::Bytes(vec![0, 171, 255])), "\\x00abff");
    }

    #[test]
    fn hex_and_quotes() {
        assert_eq!(hex(&[]), "\\x");
        assert_eq!(quoted("say \"hi\""), "\"say \"\"hi\"\"\"");
    }
}
```

Create `crates/engine/src/export/csv.rs`:
```rust
//! Comma-separated values, as RFC 4180 describes them.

use super::{Exporter, delimited, plain, quoted};
use crate::{
    result::{Column, Row},
    value::Value,
};

/// Comma-separated fields, lines ending in `\r\n`. A field holding a comma,
/// a line break or a quote is quoted.
pub struct Csv;

/// `text`, quoted when a comma, line break or quote would break the record.
fn field(text: &str) -> String {
    if text.contains([',', '\n', '\r', '"']) {
        quoted(text)
    } else {
        text.to_owned()
    }
}

impl Exporter for Csv {
    fn id(&self) -> &'static str {
        "csv"
    }

    fn title(&self) -> &'static str {
        "CSV"
    }

    fn rows(&self, columns: &[Column], rows: &[Row]) -> String {
        delimited(columns, rows, field, ",", "\r\n")
    }

    fn value(&self, value: &Value) -> String {
        field(&plain(value))
    }
}

#[cfg(test)]
mod tests {
    use super::Csv;
    use crate::{export::Exporter, result::Column, value::Value};

    fn columns(names: &[&str]) -> Vec<Column> {
        names
            .iter()
            .map(|name| Column {
                name: (*name).to_owned(),
                type_name: String::new(),
            })
            .collect()
    }

    #[test]
    fn writes_a_header_and_one_record_per_row() {
        let rows = vec![
            vec![Value::Int(1), Value::Text("Ada".to_owned()), Value::Null],
            vec![
                Value::Float(1e300),
                Value::Bytes(vec![255]),
                Value::Float(f64::INFINITY),
            ],
        ];
        assert_eq!(
            Csv.rows(&columns(&["id", "name", "note"]), &rows),
            "id,name,note\r\n1,Ada,\r\n1e300,\\xff,inf\r\n"
        );
    }

    #[test]
    fn quotes_fields_that_would_break_the_record() {
        let rows = vec![vec![
            Value::Text("a,b".to_owned()),
            Value::Text("c\nd".to_owned()),
            Value::Text("e\rf".to_owned()),
            Value::Text("say \"hi\"".to_owned()),
            Value::Text("tab\there".to_owned()),
        ]];
        assert_eq!(
            Csv.rows(&columns(&["x,y", "b", "c", "d", "e"]), &rows),
            "\"x,y\",b,c,d,e\r\n\"a,b\",\"c\nd\",\"e\rf\",\"say \"\"hi\"\"\",tab\there\r\n"
        );
    }

    #[test]
    fn no_rows_is_the_header_alone() {
        assert_eq!(Csv.rows(&columns(&["a", "b"]), &[]), "a,b\r\n");
    }

    #[test]
    fn a_single_value_is_quoted_only_when_needed() {
        assert_eq!(Csv.value(&Value::Text("a,b".to_owned())), "\"a,b\"");
        assert_eq!(Csv.value(&Value::Text("plain".to_owned())), "plain");
        assert_eq!(Csv.value(&Value::Null), "");
    }
}
```

Create `crates/engine/src/export/json.rs`:
```rust
//! JSON: an array of objects keyed by column name.

use std::{collections::HashMap, fmt::Write};

use super::{Exporter, hex};
use crate::{
    result::{Column, Row},
    value::Value,
};

/// An array of objects, one per row, keyed by column name in column order
/// and pretty-printed with two-space indents.
pub struct Json;

/// `value` as JSON: numbers stay numbers, a float that is not finite
/// becomes `null`, and bytes become their `\x…` hex as a string.
fn json(value: &Value) -> String {
    match value {
        Value::Int(n) => n.to_string(),
        Value::Float(x) if x.is_finite() => format!("{x:?}"),
        Value::Null | Value::Float(_) => "null".to_owned(),
        Value::Text(text) => string(text),
        Value::Bytes(bytes) => string(&hex(bytes)),
    }
}

/// `text` as a JSON string.
fn string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The object keys: the column names, with a repeated name numbered from
/// its second use, as in `name (2)`.
fn keys(columns: &[Column]) -> Vec<String> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    columns
        .iter()
        .map(|column| {
            let count = seen.entry(column.name.as_str()).or_insert(0);
            *count += 1;
            match *count {
                1 => column.name.clone(),
                n => format!("{} ({n})", column.name),
            }
        })
        .collect()
}

/// One row's object, indented as an array element.
fn object(keys: &[String], row: &Row) -> String {
    if keys.is_empty() {
        return "  {}".to_owned();
    }
    let fields: Vec<String> = keys
        .iter()
        .zip(row)
        .map(|(key, value)| format!("    {}: {}", string(key), json(value)))
        .collect();
    format!("  {{\n{}\n  }}", fields.join(",\n"))
}

impl Exporter for Json {
    fn id(&self) -> &'static str {
        "json"
    }

    fn title(&self) -> &'static str {
        "JSON"
    }

    fn rows(&self, columns: &[Column], rows: &[Row]) -> String {
        if rows.is_empty() {
            return "[]".to_owned();
        }
        let keys = keys(columns);
        let objects: Vec<String> = rows.iter().map(|row| object(&keys, row)).collect();
        format!("[\n{}\n]", objects.join(",\n"))
    }

    fn value(&self, value: &Value) -> String {
        json(value)
    }
}

#[cfg(test)]
mod tests {
    use super::Json;
    use crate::{export::Exporter, result::Column, value::Value};

    fn columns(names: &[&str]) -> Vec<Column> {
        names
            .iter()
            .map(|name| Column {
                name: (*name).to_owned(),
                type_name: String::new(),
            })
            .collect()
    }

    #[test]
    fn writes_an_array_of_objects_in_column_order() {
        let rows = vec![
            vec![Value::Int(1), Value::Text("Ada".to_owned()), Value::Null],
            vec![
                Value::Float(2.5),
                Value::Bytes(vec![1, 255]),
                Value::Float(1e300),
            ],
        ];
        assert_eq!(
            Json.rows(&columns(&["z", "a", "m"]), &rows),
            "[\n  {\n    \"z\": 1,\n    \"a\": \"Ada\",\n    \"m\": null\n  },\n  \
             {\n    \"z\": 2.5,\n    \"a\": \"\\\\x01ff\",\n    \"m\": 1e300\n  }\n]"
        );
    }

    #[test]
    fn floats_that_are_not_finite_are_null() {
        for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(Json.value(&Value::Float(x)), "null");
        }
        assert_eq!(Json.value(&Value::Float(-0.5)), "-0.5");
    }

    #[test]
    fn escapes_strings() {
        let text = "q\" b\\ n\n r\r t\t bell\u{7} é";
        assert_eq!(
            Json.value(&Value::Text(text.to_owned())),
            "\"q\\\" b\\\\ n\\n r\\r t\\t bell\\u0007 é\""
        );
        assert_eq!(Json.value(&Value::Text(" ~".to_owned())), "\" ~\"");
    }

    #[test]
    fn numbers_repeated_column_names() {
        let rows = vec![vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
        ]];
        assert_eq!(
            Json.rows(&columns(&["id", "id", "name", "id"]), &rows),
            "[\n  {\n    \"id\": 1,\n    \"id (2)\": 2,\n    \"name\": 3,\n    \"id (3)\": 4\n  }\n]"
        );
    }

    #[test]
    fn no_rows_is_an_empty_array() {
        assert_eq!(Json.rows(&columns(&["a"]), &[]), "[]");
    }

    #[test]
    fn a_row_without_columns_is_an_empty_object() {
        assert_eq!(Json.rows(&[], &[vec![]]), "[\n  {}\n]");
    }

    #[test]
    fn a_single_value_is_the_json_value() {
        assert_eq!(Json.value(&Value::Null), "null");
        assert_eq!(Json.value(&Value::Int(-7)), "-7");
        assert_eq!(Json.value(&Value::Text("x".to_owned())), "\"x\"");
    }
}
```

Create `crates/engine/src/export/tsv.rs`:
```rust
//! Tab-separated values: what spreadsheets paste.

use super::{Exporter, delimited, plain, quoted};
use crate::{
    result::{Column, Row},
    value::Value,
};

/// One line per row, fields separated by tabs, lines ending in `\n`.
/// A field holding a tab, a line break or a quote is quoted CSV-style.
pub struct Tsv;

/// `text`, quoted only when a tab, line break or quote would break the line.
fn field(text: &str) -> String {
    if text.contains(['\t', '\n', '\r', '"']) {
        quoted(text)
    } else {
        text.to_owned()
    }
}

impl Exporter for Tsv {
    fn id(&self) -> &'static str {
        "tsv"
    }

    fn title(&self) -> &'static str {
        "TSV"
    }

    fn rows(&self, columns: &[Column], rows: &[Row]) -> String {
        delimited(columns, rows, field, "\t", "\n")
    }

    fn value(&self, value: &Value) -> String {
        plain(value)
    }
}

#[cfg(test)]
mod tests {
    use super::Tsv;
    use crate::{export::Exporter, result::Column, value::Value};

    fn columns(names: &[&str]) -> Vec<Column> {
        names
            .iter()
            .map(|name| Column {
                name: (*name).to_owned(),
                type_name: String::new(),
            })
            .collect()
    }

    #[test]
    fn writes_a_header_and_one_line_per_row() {
        let rows = vec![
            vec![Value::Int(1), Value::Text("Ada".to_owned()), Value::Null],
            vec![
                Value::Float(2.5),
                Value::Bytes(vec![1, 2]),
                Value::Float(f64::NAN),
            ],
        ];
        assert_eq!(
            Tsv.rows(&columns(&["id", "name", "note"]), &rows),
            "id\tname\tnote\n1\tAda\t\n2.5\t\\x0102\tNaN\n"
        );
    }

    #[test]
    fn quotes_fields_that_would_break_the_line() {
        let rows = vec![vec![
            Value::Text("a\tb".to_owned()),
            Value::Text("c\nd".to_owned()),
            Value::Text("e\rf".to_owned()),
            Value::Text("say \"hi\"".to_owned()),
            Value::Text("x, y".to_owned()),
        ]];
        assert_eq!(
            Tsv.rows(&columns(&["t\tab", "b", "c", "d", "e"]), &rows),
            "\"t\tab\"\tb\tc\td\te\n\"a\tb\"\t\"c\nd\"\t\"e\rf\"\t\"say \"\"hi\"\"\"\tx, y\n"
        );
    }

    #[test]
    fn no_rows_is_the_header_alone() {
        assert_eq!(Tsv.rows(&columns(&["a", "b"]), &[]), "a\tb\n");
    }

    #[test]
    fn a_single_value_is_its_raw_text() {
        assert_eq!(
            Tsv.value(&Value::Text("a\tb \"c\"".to_owned())),
            "a\tb \"c\""
        );
        assert_eq!(Tsv.value(&Value::Null), "");
        assert_eq!(Tsv.value(&Value::Int(7)), "7");
    }
}
```

Modify `crates/engine/src/lib.rs`:
```diff
@@ -5,6 +5,7 @@
 
 pub mod driver;
 pub mod error;
+pub mod export;
 pub mod plan;
 pub mod registry;
 pub mod result;
```

- [ ] **Step 2: Run the tests and every gate**

Run: `cargo nextest run -p quarry-engine export`
Expected: 18 tests pass.

Run: `cargo mutants -p quarry-engine -f 'crates/engine/src/export*' -f 'crates/engine/src/export/*'`
Expected: 0 missed (59 caught, 4 unviable in the prototype). Then `rm -rf mutants.out mutants.out.old`.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add crates/engine/src/export.rs crates/engine/src/export/csv.rs crates/engine/src/export/json.rs crates/engine/src/export/tsv.rs crates/engine/src/lib.rs
git commit -m "feat(engine): add TSV, CSV and JSON exporters"
```

---

### Task 2: Store: schema v2 and `Drafts::closed`

`Drafts` gains `closed`: the recently closed tabs, newest first. Migration v2 adds `closed_tabs(position, title, text)`, and `save` writes it in the same transaction as the tabs. A v1 file is upgraded in place and keeps its tabs; a newer file is still refused and left byte-identical. `DraftStore` and `Forgetful` don't change. `load` and `save` are split into one small function per table to stay under the statement limit. The workspace keeps the list it loaded and writes it back in every snapshot, so saving never drops it.

**Files:**
- Modify: `crates/app/src/views/workspace/drafts.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Modify: `crates/app/tests/app/tabs.rs`
- Modify: `crates/store/src/drafts.rs`
- Modify: `crates/store/src/lib.rs`
- Modify: `crates/store/src/migrations.rs`
- Modify: `crates/store/src/sqlite.rs`
- Modify: `crates/store/tests/drafts.rs`

**Interfaces:**
- Consumes: Phase 1D's `quarry_store::{drafts, sqlite, migrations}`.
- Produces: `Drafts { tabs: Vec<Draft>, active: usize, closed: Vec<Draft> }` (each closed draft loads with id 0); `migrations::VERSION == 2`; `Workspace::snapshot` includes `closed`; the workspace has a private `closed: Vec<Draft>` field, which later tasks use.

- [ ] **Step 1: Write the failing tests**

Modify `crates/app/tests/app/tabs.rs`:
```diff
@@ -170,6 +170,23 @@ fn drafts_come_back_in_a_new_workspace(cx: &mut TestAppContext) {
     );
 }
 
+#[gpui_kit::test]
+fn the_closed_tabs_are_kept_through_a_save(cx: &mut TestAppContext) {
+    install(cx);
+    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
+    let closed = vec![draft(0, "Query 4", "SELECT 4")];
+    let saved = Drafts {
+        tabs: vec![draft(1, "Query 1", "SELECT 1")],
+        active: 0,
+        closed: closed.clone(),
+    };
+    store.save(&saved).expect("a first save");
+    let (_workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
+    cx.simulate_keystrokes("cmd-t");
+    settle(cx, |_, _| true);
+    assert_eq!(store.load().expect("the drafts load").closed, closed);
+}
+
 #[gpui_kit::test]
 fn an_edit_is_saved_only_after_a_pause(cx: &mut TestAppContext) {
     install(cx);
@@ -178,6 +195,7 @@ fn an_edit_is_saved_only_after_a_pause(cx: &mut TestAppContext) {
         .save(&Drafts {
             tabs: vec![draft(1, "Query 1", "")],
             active: 0,
+            closed: Vec::new(),
         })
         .expect("a first save");
     let (_workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
```

Modify `crates/store/tests/drafts.rs`:
```diff
@@ -21,6 +21,10 @@ fn sample() -> Drafts {
             draft(3, "Query 1", "SELECT 1;\n-- é ✓"),
         ],
         active: 1,
+        closed: vec![
+            draft(0, "Notes", "-- newest"),
+            draft(0, "Query 4", "SELECT 4;"),
+        ],
     }
 }
 
@@ -44,6 +48,7 @@ fn a_save_replaces_the_previous_one() {
     let fewer = Drafts {
         tabs: vec![draft(9, "Query 5", "")],
         active: 0,
+        closed: Vec::new(),
     };
     store.save(&fewer).unwrap();
     assert_eq!(store.load().unwrap(), fewer);
@@ -81,7 +86,7 @@ fn leaves_a_newer_file_untouched() {
         error,
         Some(StoreError::TooNew {
             found: 5,
-            supported: 1
+            supported: 2
         })
     );
     assert_eq!(std::fs::read(&path).unwrap(), before);
@@ -104,3 +109,63 @@ fn reports_a_file_that_is_not_a_database() {
     let error = SqliteStore::open(&path).err();
     assert!(matches!(error, Some(StoreError::Sql(_))), "{error:?}");
 }
+
+#[test]
+fn closed_tabs_round_trip_newest_first_and_an_empty_list_too() {
+    let store = SqliteStore::in_memory().unwrap();
+    store.save(&sample()).unwrap();
+    let closed = store.load().unwrap().closed;
+    assert_eq!(closed, sample().closed);
+    let mut none = sample();
+    none.closed.clear();
+    store.save(&none).unwrap();
+    assert_eq!(store.load().unwrap(), none);
+}
+
+#[test]
+fn a_closed_tab_loads_without_its_id() {
+    let store = SqliteStore::in_memory().unwrap();
+    let mut drafts = sample();
+    drafts.closed = vec![draft(42, "Query 9", "SELECT 9;")];
+    store.save(&drafts).unwrap();
+    assert_eq!(
+        store.load().unwrap().closed,
+        [draft(0, "Query 9", "SELECT 9;")]
+    );
+}
+
+#[test]
+fn a_version_one_file_is_upgraded_and_keeps_its_tabs() {
+    let dir = tempfile::tempdir().unwrap();
+    let path = dir.path().join("quarry.db");
+    let connection = rusqlite::Connection::open(&path).unwrap();
+    connection
+        .execute_batch(
+            "CREATE TABLE drafts (id INTEGER PRIMARY KEY, position INTEGER NOT NULL,
+                 title TEXT NOT NULL, text TEXT NOT NULL);
+             CREATE TABLE state (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
+             INSERT INTO drafts VALUES (5, 0, 'Query 1', 'SELECT 1;');
+             INSERT INTO state VALUES ('active', 0);
+             PRAGMA user_version = 1;",
+        )
+        .unwrap();
+    drop(connection);
+    let loaded = SqliteStore::open(&path).unwrap().load().unwrap();
+    assert_eq!(
+        loaded,
+        Drafts {
+            tabs: vec![draft(5, "Query 1", "SELECT 1;")],
+            active: 0,
+            closed: Vec::new(),
+        }
+    );
+}
+
+#[test]
+fn keeps_the_first_tab_active() {
+    let store = SqliteStore::in_memory().unwrap();
+    let mut drafts = sample();
+    drafts.active = 0;
+    store.save(&drafts).unwrap();
+    assert_eq!(store.load().unwrap().active, 0);
+}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry-store`
Expected: FAIL to compile: `struct Drafts has no field named closed`.

- [ ] **Step 3: Implement**

Modify `crates/app/src/views/workspace/drafts.rs`:
```diff
@@ -61,7 +61,8 @@ impl Workspace {
             .or(self.open_problem.as_deref())
     }
 
-    /// Every tab's title and text, and which one is active.
+    /// Every tab's title and text, which one is active, and the recently
+    /// closed tabs.
     pub fn snapshot(&self, cx: &App) -> Drafts {
         let tabs = self
             .tabs
@@ -78,6 +79,7 @@ impl Workspace {
         Drafts {
             tabs,
             active: self.tabs.active(),
+            closed: self.closed.clone(),
         }
     }
 
```

Modify `crates/app/src/views/workspace/mod.rs`:
```diff
@@ -17,7 +17,7 @@ use gpui_kit::{
     component::input::EditorState,
 };
 use quarry_engine::session::Session;
-use quarry_store::drafts::{DraftStore, Drafts};
+use quarry_store::drafts::{Draft, DraftStore, Drafts};
 use quarry_ui_kit::Aurora;
 
 pub use self::{
@@ -47,6 +47,7 @@ pub struct Workspace {
     database: String,
     tabs: TabList,
     views: BTreeMap<TabId, Entity<QueryTab>>,
+    closed: Vec<Draft>,
     store: Arc<dyn DraftStore>,
     open_problem: Option<String>,
     save_problem: Option<String>,
@@ -76,6 +77,7 @@ impl Workspace {
             database: database.to_owned(),
             tabs: restored_tabs(&drafts),
             views: BTreeMap::new(),
+            closed: drafts.closed.clone(),
             store,
             open_problem: problem,
             save_problem: None,
```

Modify `crates/store/src/drafts.rs`:
```diff
@@ -13,13 +13,17 @@ pub struct Draft {
     pub text: String,
 }
 
-/// Every open tab in order, and which one is active.
+/// Every open tab in order, which one is active, and the recently closed
+/// tabs.
 #[derive(Debug, Clone, Default, PartialEq, Eq)]
 pub struct Drafts {
     /// The tabs, left to right.
     pub tabs: Vec<Draft>,
     /// The index of the active tab in `tabs`.
     pub active: usize,
+    /// Recently closed tabs, newest first. Their ids are not kept: each
+    /// loads with id 0.
+    pub closed: Vec<Draft>,
 }
 
 /// Loads and saves the drafts.
@@ -66,6 +70,7 @@ mod tests {
                 text: "SELECT 1".to_owned(),
             }],
             active: 0,
+            closed: Vec::new(),
         };
         store.save(&drafts)?;
         assert_eq!(store.load()?, Drafts::default());
```

Modify `crates/store/src/lib.rs`:
```diff
@@ -1,7 +1,8 @@
 //! Local persistence: settings, connections, history, and progress.
 //!
-//! Phase 1D stores the query tabs' drafts. Each kind of data gets a trait
-//! ([`drafts::DraftStore`]) so the app can swap in a fake or a fallback.
+//! It stores the query tabs' drafts and the recently closed tabs. Each kind
+//! of data gets a trait ([`drafts::DraftStore`]) so the app can swap in a
+//! fake or a fallback.
 
 pub mod drafts;
 pub mod error;
```

Modify `crates/store/src/migrations.rs`:
```diff
@@ -5,16 +5,23 @@ use rusqlite::Connection;
 use crate::error::StoreError;
 
 /// Each step moves the schema up one version; step `n` makes version `n + 1`.
-const STEPS: &[&str] = &["CREATE TABLE drafts (
+const STEPS: &[&str] = &[
+    "CREATE TABLE drafts (
         id INTEGER PRIMARY KEY,
         position INTEGER NOT NULL,
         title TEXT NOT NULL,
         text TEXT NOT NULL
     );
-    CREATE TABLE state (key TEXT PRIMARY KEY, value INTEGER NOT NULL);"];
+    CREATE TABLE state (key TEXT PRIMARY KEY, value INTEGER NOT NULL);",
+    "CREATE TABLE closed_tabs (
+        position INTEGER NOT NULL,
+        title TEXT NOT NULL,
+        text TEXT NOT NULL
+    );",
+];
 
 /// The schema version this build writes.
-pub const VERSION: i64 = 1;
+pub const VERSION: i64 = 2;
 
 /// Brings the schema up to [`VERSION`] in one transaction.
 ///
@@ -64,12 +71,35 @@ mod tests {
         migrate(&mut connection).unwrap();
         let tables: i64 = connection
             .query_row(
-                "SELECT count(*) FROM sqlite_schema WHERE name IN ('drafts', 'state')",
+                "SELECT count(*) FROM sqlite_schema WHERE name IN ('drafts', 'state', 'closed_tabs')",
                 [],
                 |row| row.get(0),
             )
             .unwrap();
-        assert_eq!(tables, 2);
+        assert_eq!(tables, 3);
+    }
+
+    #[test]
+    fn upgrades_a_version_one_database_in_place() {
+        let mut connection = Connection::open_in_memory().unwrap();
+        connection.execute_batch(STEPS[0]).unwrap();
+        connection.pragma_update(None, "user_version", 1).unwrap();
+        connection
+            .execute(
+                "INSERT INTO drafts (id, position, title, text) VALUES (4, 0, 'Query 1', 'SELECT 1')",
+                [],
+            )
+            .unwrap();
+        migrate(&mut connection).unwrap();
+        assert_eq!(version(&connection), 2);
+        let kept: String = connection
+            .query_row("SELECT text FROM drafts WHERE id = 4", [], |row| row.get(0))
+            .unwrap();
+        assert_eq!(kept, "SELECT 1");
+        let closed: i64 = connection
+            .query_row("SELECT count(*) FROM closed_tabs", [], |row| row.get(0))
+            .unwrap();
+        assert_eq!(closed, 0);
     }
 
     #[test]
```

Modify `crates/store/src/sqlite.rs`:
```diff
@@ -63,42 +63,85 @@ fn index(position: usize) -> i64 {
     i64::try_from(position).unwrap_or(i64::MAX)
 }
 
+/// The open tabs, left to right.
+fn load_tabs(connection: &Connection) -> Result<Vec<Draft>, StoreError> {
+    let mut statement =
+        connection.prepare("SELECT id, title, text FROM drafts ORDER BY position")?;
+    let tabs = statement.query_map([], |row| {
+        Ok(Draft {
+            id: row.get(0)?,
+            title: row.get(1)?,
+            text: row.get(2)?,
+        })
+    })?;
+    Ok(tabs.collect::<Result<Vec<_>, _>>()?)
+}
+
+/// The recently closed tabs, newest first, each with id 0.
+fn load_closed(connection: &Connection) -> Result<Vec<Draft>, StoreError> {
+    let mut statement =
+        connection.prepare("SELECT title, text FROM closed_tabs ORDER BY position")?;
+    let closed = statement.query_map([], |row| {
+        Ok(Draft {
+            id: 0,
+            title: row.get(0)?,
+            text: row.get(1)?,
+        })
+    })?;
+    Ok(closed.collect::<Result<Vec<_>, _>>()?)
+}
+
+/// The saved active index, or 0 when none was saved.
+fn load_active(connection: &Connection) -> Result<usize, StoreError> {
+    let active: Option<i64> = connection
+        .query_row("SELECT value FROM state WHERE key = 'active'", [], |row| {
+            row.get(0)
+        })
+        .optional()?;
+    Ok(usize::try_from(active.unwrap_or(0)).unwrap_or(0))
+}
+
+/// Replaces the open tabs.
+fn save_tabs(connection: &Connection, tabs: &[Draft]) -> Result<(), StoreError> {
+    connection.execute("DELETE FROM drafts", [])?;
+    for (position, draft) in tabs.iter().enumerate() {
+        connection.execute(
+            "INSERT INTO drafts (id, position, title, text) VALUES (?1, ?2, ?3, ?4)",
+            params![draft.id, index(position), draft.title, draft.text],
+        )?;
+    }
+    Ok(())
+}
+
+/// Replaces the recently closed tabs.
+fn save_closed(connection: &Connection, closed: &[Draft]) -> Result<(), StoreError> {
+    connection.execute("DELETE FROM closed_tabs", [])?;
+    for (position, draft) in closed.iter().enumerate() {
+        connection.execute(
+            "INSERT INTO closed_tabs (position, title, text) VALUES (?1, ?2, ?3)",
+            params![index(position), draft.title, draft.text],
+        )?;
+    }
+    Ok(())
+}
+
 impl DraftStore for SqliteStore {
     fn load(&self) -> Result<Drafts, StoreError> {
         let connection = self.connection();
-        let mut statement =
-            connection.prepare("SELECT id, title, text FROM drafts ORDER BY position")?;
-        let tabs = statement
-            .query_map([], |row| {
-                Ok(Draft {
-                    id: row.get(0)?,
-                    title: row.get(1)?,
-                    text: row.get(2)?,
-                })
-            })?
-            .collect::<Result<Vec<_>, _>>()?;
-        let active: Option<i64> = connection
-            .query_row("SELECT value FROM state WHERE key = 'active'", [], |row| {
-                row.get(0)
-            })
-            .optional()?;
-        let active = usize::try_from(active.unwrap_or(0)).unwrap_or(0);
+        let tabs = load_tabs(&connection)?;
+        let active = load_active(&connection)?;
         Ok(Drafts {
             active: active.min(tabs.len().saturating_sub(1)),
             tabs,
+            closed: load_closed(&connection)?,
         })
     }
 
     fn save(&self, drafts: &Drafts) -> Result<(), StoreError> {
         let mut connection = self.connection();
         let transaction = connection.transaction()?;
-        transaction.execute("DELETE FROM drafts", [])?;
-        for (position, draft) in drafts.tabs.iter().enumerate() {
-            transaction.execute(
-                "INSERT INTO drafts (id, position, title, text) VALUES (?1, ?2, ?3, ?4)",
-                params![draft.id, index(position), draft.title, draft.text],
-            )?;
-        }
+        save_tabs(&transaction, &drafts.tabs)?;
+        save_closed(&transaction, &drafts.closed)?;
         transaction.execute(
             "INSERT OR REPLACE INTO state (key, value) VALUES ('active', ?1)",
             params![index(drafts.active)],
```

- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry-store -p quarry`
Expected: every test passes, including `a_version_one_file_is_upgraded_and_keeps_its_tabs`, `closed_tabs_round_trip_newest_first_and_an_empty_list_too`, `upgrades_a_version_one_database_in_place` and `the_closed_tabs_are_kept_through_a_save`.

Run: `cargo mutants -p quarry-store`
Expected: 0 missed. (`keeps_the_first_tab_active` is there because the prototype's run missed `load_active -> Ok(1)` without it.) Then `rm -rf mutants.out mutants.out.old`.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/app/src/views/workspace/drafts.rs crates/app/src/views/workspace/mod.rs crates/app/tests/app/tabs.rs crates/store/src/drafts.rs crates/store/src/lib.rs crates/store/src/migrations.rs crates/store/src/sqlite.rs crates/store/tests/drafts.rs
git commit -m "feat(store): keep recently closed tabs in schema v2"
```

---

### Task 3: Models: `palette`, `copy`, `closed_tabs`, `save_order`

Four GPUI-free models, each with every rule in spec §3.3 under test:
- **`palette::filter`:** case-insensitive matching on the label. A prefix ranks first, then a word start, then anywhere. Ties keep each source's order, sources keep theirs, and at most 50 results come back.
- **`copy`:** turns a grid selection into rows or one value, and writes the status-bar note.
- **`closed_tabs::remember` and `take`:** ignore blank text, move a repeat to the front, and cap the list at 10.
- **`SaveOrder`:** hands out rising generations and refuses one older than the last written.

**Files:**
- Create: `crates/app/src/model/closed_tabs.rs`
- Create: `crates/app/src/model/copy.rs`
- Modify: `crates/app/src/model/mod.rs`
- Create: `crates/app/src/model/palette.rs`
- Create: `crates/app/src/model/save_order.rs`

**Interfaces:**
- Consumes: `quarry_engine::export::Exporter` (Task 1), `quarry_store::drafts::Draft`, `model::format::{count, rows}`.
- Produces:
  - `model::palette::{PALETTE_LIMIT = 50, PaletteItem { source: &'static str, key: String, label: String, detail: String }, filter(&[PaletteItem], &str) -> Vec<&PaletteItem>}`
  - `model::copy::{Selection::{None, Row(usize), Column(usize), Cell(usize, usize)}, Copied::{Rows { columns: Vec<Column>, rows: Vec<Row>, of: u64 }, Value(Value)}, copied(&[Column], &[Row], received: u64, Selection) -> Option<Copied>, Copied::text(&self, &dyn Exporter) -> String, note(Option<&Copied>, format: &str) -> String}`
  - `model::closed_tabs::{CLOSED_LIMIT = 10, remember(&mut Vec<Draft>, Draft), take(&mut Vec<Draft>, usize) -> Option<Draft>}`
  - `model::save_order::SaveOrder::{default(), issue(&mut self) -> u64, commit(&mut self, u64) -> bool}`

- [ ] **Step 1: Implement**

Create `crates/app/src/model/closed_tabs.rs`:
```rust
//! The recently closed tabs, newest first, for reopening from ⌘K.

use quarry_store::drafts::Draft;

/// The most closed tabs Quarry remembers.
pub const CLOSED_LIMIT: usize = 10;

/// Puts `draft` at the front of `closed`. A tab with nothing but blank text
/// is not worth reopening and is ignored; one with the same title and text
/// as an entry already there moves that entry to the front. The list keeps
/// at most [`CLOSED_LIMIT`] entries.
pub fn remember(closed: &mut Vec<Draft>, draft: Draft) {
    if draft.text.trim().is_empty() {
        return;
    }
    closed.retain(|known| known.title != draft.title || known.text != draft.text);
    closed.insert(0, draft);
    closed.truncate(CLOSED_LIMIT);
}

/// Removes and returns entry `index`, if there is one.
pub fn take(closed: &mut Vec<Draft>, index: usize) -> Option<Draft> {
    (index < closed.len()).then(|| closed.remove(index))
}

#[cfg(test)]
mod tests {
    use quarry_store::drafts::Draft;

    use super::{CLOSED_LIMIT, remember, take};

    fn draft(title: &str, text: &str) -> Draft {
        Draft {
            id: 0,
            title: title.to_owned(),
            text: text.to_owned(),
        }
    }

    fn titles(closed: &[Draft]) -> Vec<&str> {
        closed.iter().map(|draft| draft.title.as_str()).collect()
    }

    #[test]
    fn the_newest_comes_first() {
        let mut closed = Vec::new();
        remember(&mut closed, draft("Query 1", "SELECT 1"));
        remember(&mut closed, draft("Query 2", "SELECT 2"));
        assert_eq!(titles(&closed), ["Query 2", "Query 1"]);
    }

    #[test]
    fn blank_text_is_ignored() {
        let mut closed = Vec::new();
        remember(&mut closed, draft("Query 1", ""));
        remember(&mut closed, draft("Query 2", " \n\t "));
        assert!(closed.is_empty());
    }

    #[test]
    fn the_same_tab_again_moves_to_the_front() {
        let mut closed = vec![draft("Query 1", "SELECT 1"), draft("Query 2", "SELECT 2")];
        remember(&mut closed, draft("Query 2", "SELECT 2"));
        assert_eq!(titles(&closed), ["Query 2", "Query 1"]);
    }

    #[test]
    fn a_different_title_or_text_is_a_different_tab() {
        let mut closed = vec![draft("Query 1", "SELECT 1")];
        remember(&mut closed, draft("Query 1", "SELECT 2"));
        remember(&mut closed, draft("Query 9", "SELECT 1"));
        assert_eq!(closed.len(), 3);
    }

    #[test]
    fn at_most_the_limit_is_kept() {
        let mut closed = Vec::new();
        for n in 0..=CLOSED_LIMIT {
            remember(&mut closed, draft(&format!("Query {n}"), "SELECT 1"));
        }
        assert_eq!(closed.len(), CLOSED_LIMIT);
        assert_eq!(closed[0].title, format!("Query {CLOSED_LIMIT}"));
        assert_eq!(closed[CLOSED_LIMIT - 1].title, "Query 1");
    }

    #[test]
    fn take_removes_one_entry() {
        let mut closed = vec![draft("Query 1", "a"), draft("Query 2", "b")];
        assert_eq!(take(&mut closed, 1), Some(draft("Query 2", "b")));
        assert_eq!(take(&mut closed, 1), None);
        assert_eq!(titles(&closed), ["Query 1"]);
    }
}
```

Create `crates/app/src/model/copy.rs`:
```rust
//! What a copy takes from the grid, and the note that says what it took.

use quarry_engine::{
    export::Exporter,
    result::{Column, Row},
    value::Value,
};

use super::format;

/// What is selected in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// Nothing: a copy takes every kept row.
    None,
    /// One row.
    Row(usize),
    /// One column.
    Column(usize),
    /// One cell, as (row, column).
    Cell(usize, usize),
}

/// What a copy puts on the clipboard, before it is formatted.
#[derive(Debug, Clone, PartialEq)]
pub enum Copied {
    /// Rows under a header.
    Rows {
        /// The header.
        columns: Vec<Column>,
        /// The rows copied.
        rows: Vec<Row>,
        /// How many rows the copied rows stand for: more than `rows.len()`
        /// when the statement streamed more rows than Quarry keeps.
        of: u64,
    },
    /// A single value.
    Value(Value),
}

/// What copying `selection` takes from a statement's `columns` and kept
/// `rows`, of the `received` rows it streamed. `None` when there is
/// nothing to copy: no rows, or an index out of range.
pub fn copied(
    columns: &[Column],
    rows: &[Row],
    received: u64,
    selection: Selection,
) -> Option<Copied> {
    match selection {
        Selection::None => whole(columns, rows, received),
        Selection::Row(row) => Some(Copied::Rows {
            columns: columns.to_vec(),
            rows: vec![rows.get(row)?.clone()],
            of: 1,
        }),
        Selection::Column(column) => Some(Copied::Rows {
            columns: vec![columns.get(column)?.clone()],
            rows: column_values(rows, column)?,
            of: received,
        }),
        Selection::Cell(row, column) => Some(Copied::Value(rows.get(row)?.get(column)?.clone())),
    }
}

fn whole(columns: &[Column], rows: &[Row], received: u64) -> Option<Copied> {
    (!rows.is_empty()).then(|| Copied::Rows {
        columns: columns.to_vec(),
        rows: rows.to_vec(),
        of: received,
    })
}

/// Each row's value in `column`, as a one-value row; `None` without rows.
fn column_values(rows: &[Row], column: usize) -> Option<Vec<Row>> {
    let values: Option<Vec<Row>> = rows
        .iter()
        .map(|row| Some(vec![row.get(column)?.clone()]))
        .collect();
    values.filter(|values| !values.is_empty())
}

impl Copied {
    /// The text `exporter` makes of it.
    pub fn text(&self, exporter: &dyn Exporter) -> String {
        match self {
            Self::Rows { columns, rows, .. } => exporter.rows(columns, rows),
            Self::Value(value) => exporter.value(value),
        }
    }
}

/// The status-bar note after a copy as `format`, such as "Copied 12 rows
/// as CSV", or "Nothing to copy".
pub fn note(copied: Option<&Copied>, format: &str) -> String {
    match copied {
        None => "Nothing to copy".to_owned(),
        Some(Copied::Value(_)) => format!("Copied 1 value as {format}"),
        Some(Copied::Rows { rows, of, .. }) => {
            let kept = u64::try_from(rows.len()).unwrap_or(u64::MAX);
            if *of > kept {
                let (kept, of) = (format::count(kept), format::count(*of));
                format!("Copied {kept} of {of} rows as {format}")
            } else {
                format!("Copied {} as {format}", format::rows(kept))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quarry_engine::{
        export::ExporterRegistry,
        result::{Column, Row},
        value::Value,
    };

    use super::{Copied, Selection, copied, note};

    fn columns() -> Vec<Column> {
        ["id", "name"]
            .iter()
            .map(|name| Column {
                name: (*name).to_owned(),
                type_name: String::new(),
            })
            .collect()
    }

    fn rows() -> Vec<Row> {
        vec![
            vec![Value::Int(1), Value::Text("Ada".to_owned())],
            vec![Value::Int(2), Value::Null],
        ]
    }

    #[test]
    fn nothing_selected_copies_every_kept_row() {
        assert_eq!(
            copied(&columns(), &rows(), 2, Selection::None),
            Some(Copied::Rows {
                columns: columns(),
                rows: rows(),
                of: 2
            })
        );
    }

    #[test]
    fn a_row_copies_the_header_and_that_row() {
        assert_eq!(
            copied(&columns(), &rows(), 9, Selection::Row(1)),
            Some(Copied::Rows {
                columns: columns(),
                rows: vec![rows()[1].clone()],
                of: 1
            })
        );
    }

    #[test]
    fn a_column_copies_its_header_and_values() {
        assert_eq!(
            copied(&columns(), &rows(), 2, Selection::Column(1)),
            Some(Copied::Rows {
                columns: vec![columns()[1].clone()],
                rows: vec![vec![Value::Text("Ada".to_owned())], vec![Value::Null]],
                of: 2
            })
        );
    }

    #[test]
    fn a_cell_copies_its_value_alone() {
        assert_eq!(
            copied(&columns(), &rows(), 2, Selection::Cell(0, 1)),
            Some(Copied::Value(Value::Text("Ada".to_owned())))
        );
    }

    #[test]
    fn nothing_is_copied_without_rows_or_out_of_range() {
        let none = |selection| copied(&columns(), &rows(), 2, selection);
        assert_eq!(copied(&columns(), &[], 0, Selection::None), None);
        assert_eq!(copied(&columns(), &[], 0, Selection::Column(0)), None);
        assert_eq!(none(Selection::Row(2)), None);
        assert_eq!(none(Selection::Column(2)), None);
        assert_eq!(none(Selection::Cell(2, 0)), None);
        assert_eq!(none(Selection::Cell(0, 2)), None);
    }

    #[test]
    fn text_is_what_the_exporter_makes() {
        let registry = ExporterRegistry::standard();
        let tsv = registry.get("tsv").unwrap();
        let rows = copied(&columns(), &rows(), 2, Selection::None).unwrap();
        assert_eq!(rows.text(tsv), "id\tname\n1\tAda\n2\t\n");
        assert_eq!(Copied::Value(Value::Int(7)).text(tsv), "7");
    }

    #[test]
    fn notes_say_what_was_copied() {
        let rows_of = |kept: usize, of: u64| Copied::Rows {
            columns: columns(),
            rows: vec![vec![Value::Null]; kept],
            of,
        };
        assert_eq!(note(None, "TSV"), "Nothing to copy");
        assert_eq!(
            note(Some(&Copied::Value(Value::Null)), "JSON"),
            "Copied 1 value as JSON"
        );
        assert_eq!(note(Some(&rows_of(12, 12)), "CSV"), "Copied 12 rows as CSV");
        assert_eq!(note(Some(&rows_of(1, 1)), "CSV"), "Copied 1 row as CSV");
        assert_eq!(
            note(Some(&rows_of(1000, 2500)), "TSV"),
            "Copied 1,000 of 2,500 rows as TSV"
        );
    }
}
```

Modify `crates/app/src/model/mod.rs`:
```diff
@@ -3,10 +3,14 @@
 //! Nothing here depends on GPUI, so every rule is unit-tested and
 //! mutation-tested like the core crates.
 
+pub mod closed_tabs;
+pub mod copy;
 pub mod drain;
 pub mod format;
 pub mod messages;
+pub mod palette;
 pub mod plan_tree;
+pub mod save_order;
 pub mod schema_tree;
 pub mod statements;
 pub mod status;
```

Create `crates/app/src/model/palette.rs`:
```rust
//! What the ⌘K palette lists, and how typing narrows it.

/// The most results the palette shows.
pub const PALETTE_LIMIT: usize = 50;

/// One thing the palette can pick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteItem {
    /// The id of the source that listed it, such as "tabs".
    pub source: &'static str,
    /// What the source needs to find it again when it is picked.
    pub key: String,
    /// The text shown and searched.
    pub label: String,
    /// Quieter text after the label, such as "current".
    pub detail: String,
}

/// How well `label` matches `query`, both already lowercase: 0 for a
/// prefix, 1 for the start of a later word, 2 for anywhere else.
fn rank(label: &str, query: &str) -> Option<u8> {
    let mut best = None;
    for (at, _) in label.match_indices(query) {
        let word_start = label[..at]
            .chars()
            .next_back()
            .is_none_or(|before| !before.is_alphanumeric());
        let this = match (at, word_start) {
            (0, _) => 0,
            (_, true) => 1,
            _ => 2,
        };
        best = Some(best.map_or(this, |known: u8| known.min(this)));
    }
    best
}

/// The items whose label holds `query`, ignoring case, best first within
/// each source; sources keep their order, and so do equal matches. An empty
/// query keeps every item. At most [`PALETTE_LIMIT`] come back.
pub fn filter<'a>(items: &'a [PaletteItem], query: &str) -> Vec<&'a PaletteItem> {
    let query = query.to_lowercase();
    let group = |source: &str| items.iter().position(|item| item.source == source);
    let mut matched: Vec<(Option<usize>, u8, &PaletteItem)> = items
        .iter()
        .filter_map(|item| {
            let rank = rank(&item.label.to_lowercase(), &query)?;
            Some((group(item.source), rank, item))
        })
        .collect();
    matched.sort_by_key(|(group, rank, _)| (*group, *rank));
    matched
        .into_iter()
        .take(PALETTE_LIMIT)
        .map(|(_, _, item)| item)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{PALETTE_LIMIT, PaletteItem, filter, rank};

    fn item(source: &'static str, label: &str) -> PaletteItem {
        PaletteItem {
            source,
            key: label.to_owned(),
            label: label.to_owned(),
            detail: String::new(),
        }
    }

    fn labels(found: &[&PaletteItem]) -> Vec<String> {
        found.iter().map(|item| item.label.clone()).collect()
    }

    #[test]
    fn ranks_a_prefix_then_a_word_start_then_anywhere() {
        assert_eq!(rank("run all", "run"), Some(0));
        assert_eq!(rank("toggle sidebar", "side"), Some(1));
        assert_eq!(rank("toggle sidebar", "debar"), Some(2));
        assert_eq!(rank("focus sidebar", "sidebar"), Some(1));
        assert_eq!(rank("crew_members", "members"), Some(1));
        assert_eq!(rank("astronauts", "naut"), Some(2));
        assert_eq!(rank("astronauts", "x"), None);
    }

    #[test]
    fn the_best_of_several_matches_counts() {
        assert_eq!(rank("an anagram", "an"), Some(0));
        assert_eq!(rank("banana nap", "na"), Some(1));
    }

    #[test]
    fn an_empty_query_keeps_every_item_in_order() {
        let items = [
            item("tabs", "Query 2"),
            item("tabs", "Query 1"),
            item("commands", "Run All"),
        ];
        assert_eq!(
            labels(&filter(&items, "")),
            ["Query 2", "Query 1", "Run All"]
        );
    }

    #[test]
    fn matching_ignores_case() {
        let items = [item("tables", "Missions"), item("tables", "crew")];
        assert_eq!(labels(&filter(&items, "MISSION")), ["Missions"]);
    }

    #[test]
    fn better_matches_come_first_within_a_source_and_ties_keep_order() {
        let items = [
            item("commands", "Toggle Sidebar"),
            item("commands", "Focus Sidebar"),
            item("commands", "Sidebar Width"),
            item("commands", "Sidebar Shown"),
        ];
        assert_eq!(
            labels(&filter(&items, "sidebar")),
            [
                "Sidebar Width",
                "Sidebar Shown",
                "Toggle Sidebar",
                "Focus Sidebar"
            ]
        );
    }

    #[test]
    fn sources_keep_their_order_and_empty_ones_drop_out() {
        let items = [
            item("tabs", "Notes about runs"),
            item("closed", "Old"),
            item("tables", "runways"),
            item("commands", "Run All"),
        ];
        assert_eq!(
            labels(&filter(&items, "run")),
            ["Notes about runs", "runways", "Run All"]
        );
    }

    #[test]
    fn a_later_source_never_moves_ahead_of_an_earlier_one() {
        let items = [item("tabs", "mission notes"), item("tables", "missions")];
        assert_eq!(
            labels(&filter(&items, "mission")),
            ["mission notes", "missions"]
        );
        let items = [item("tables", "my missions"), item("commands", "missions")];
        assert_eq!(
            labels(&filter(&items, "mission")),
            ["my missions", "missions"]
        );
    }

    #[test]
    fn at_most_the_limit_comes_back() {
        let items: Vec<PaletteItem> = (0..PALETTE_LIMIT + 5)
            .map(|n| item("tables", &format!("t{n}")))
            .collect();
        let found = filter(&items, "");
        assert_eq!(found.len(), PALETTE_LIMIT);
        assert_eq!(
            found[PALETTE_LIMIT - 1].label,
            format!("t{}", PALETTE_LIMIT - 1)
        );
        assert_eq!(filter(&items, "t").len(), PALETTE_LIMIT);
    }
}
```

Create `crates/app/src/model/save_order.rs`:
```rust
//! Keeps saves in order: an older snapshot never lands after a newer one.

/// Hands out a rising generation per snapshot and remembers the newest one
/// written.
#[derive(Debug, Default)]
pub struct SaveOrder {
    issued: u64,
    committed: u64,
}

impl SaveOrder {
    /// The generation of a snapshot taken now: one more than the last.
    pub fn issue(&mut self) -> u64 {
        self.issued += 1;
        self.issued
    }

    /// Records that `generation` is being written; false, and nothing
    /// recorded, when a newer one already was.
    pub fn commit(&mut self, generation: u64) -> bool {
        if generation < self.committed {
            return false;
        }
        self.committed = generation;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::SaveOrder;

    #[test]
    fn generations_rise_from_one() {
        let mut saves = SaveOrder::default();
        assert_eq!(saves.issue(), 1);
        assert_eq!(saves.issue(), 2);
    }

    #[test]
    fn an_older_generation_is_refused_after_a_newer_one() {
        let mut saves = SaveOrder::default();
        let older = saves.issue();
        let newer = saves.issue();
        assert!(saves.commit(newer));
        assert!(!saves.commit(older));
        assert!(saves.commit(newer));
        assert!(saves.commit(saves.issued + 1));
    }

    #[test]
    fn generations_written_in_order_are_all_accepted() {
        let mut saves = SaveOrder::default();
        let first = saves.issue();
        let second = saves.issue();
        assert!(saves.commit(first));
        assert!(saves.commit(second));
        assert!(!saves.commit(first));
    }
}
```

- [ ] **Step 2: Run the tests and every gate**

Run: `cargo nextest run -p quarry --lib model::palette model::copy model::closed_tabs model::save_order`
Expected: every test passes.

Run: `cargo mutants -p quarry -f crates/app/src/model/palette.rs -f crates/app/src/model/copy.rs -f crates/app/src/model/closed_tabs.rs -f crates/app/src/model/save_order.rs`
Expected: 0 missed (39 caught, 4 unviable in the prototype). Then `rm -rf mutants.out mutants.out.old`.

Run: `cargo llvm-cov nextest -p quarry --lib --summary-only | grep -E 'model/(palette|copy|closed_tabs|save_order)'`
Expected: 100.00% lines for all four.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add crates/app/src/model/closed_tabs.rs crates/app/src/model/copy.rs crates/app/src/model/mod.rs crates/app/src/model/palette.rs crates/app/src/model/save_order.rs
git commit -m "feat(app): add palette, copy, closed-tab and save-order models"
```

---

### Task 4: `poll_reply`, `settle()` reporting timeouts, and the tests 1D left out

The schema read and Explain each had their own loop for polling a one-off reply. They now share `views::poll::poll_reply`. The test helper `settle()` now returns whether its condition held, and every caller asserts it, so a timeout fails the test instead of passing silently. The five view tests 1D left out are added:
- ⌘↵ from Messages returns to Results.
- A Run all with one row-returning statement shows no result-tab strip.
- The failure banner shows over Messages.
- A second ⌘E while a plan is pending replaces the first.
- ⌘W activates the right-hand neighbour.

**Files:**
- Modify: `crates/app/src/views/mod.rs`
- Create: `crates/app/src/views/poll.rs`
- Modify: `crates/app/src/views/query_tab/explain.rs`
- Modify: `crates/app/src/views/workspace/runs.rs`
- Modify: `crates/app/tests/app/runs.rs`
- Modify: `crates/app/tests/app/support.rs`
- Modify: `crates/app/tests/app/tabs.rs`
- Modify: `crates/app/tests/app/workspace.rs`

**Interfaces:**
- Consumes: `model::drain::{drain, reply}`, `views::workspace::POLL`.
- Produces: `views::poll::poll_reply<T>(Receiver<Result<T, EngineError>>, BackgroundExecutor) -> Result<T, EngineError>` (async). In tests, `support::settle(cx, done) -> bool` is `#[must_use]`.

- [ ] **Step 1: Implement**

`poll_reply` replaces the two hand-written polling loops. The five new tests cover behavior 1D already has, so there is no red step: they guard that behavior through the refactor. The Explain test proves the pending plan is dropped: its receiver closes, which `settle` waits for, because a dropped task frees its future on the next executor turn. Every existing `settle(...)` call becomes `assert!(settle(...))`.

Modify `crates/app/src/views/mod.rs`:
```diff
@@ -1,6 +1,7 @@
 //! GPUI views: each renders one region of the workspace.
 
 pub mod editor;
+pub mod poll;
 pub mod query_tab;
 pub mod results;
 pub mod schema;
```

Create `crates/app/src/views/poll.rs`:
```rust
//! Waits for a one-off reply from the session, such as a schema or a plan.
//!
//! The reply is polled once per [`POLL`] rather than awaited (see
//! [`crate::model::drain`]).

use async_channel::Receiver;
use gpui_kit::BackgroundExecutor;
use quarry_engine::error::EngineError;

use super::workspace::POLL;
use crate::model::drain;

/// The answer on `reply`, polled every [`POLL`] on `executor`'s clock.
///
/// # Errors
/// Returns the session's error, or [`EngineError::WorkerGone`] when the
/// channel closes without an answer.
pub async fn poll_reply<T>(
    reply: Receiver<Result<T, EngineError>>,
    executor: BackgroundExecutor,
) -> Result<T, EngineError> {
    loop {
        executor.timer(POLL).await;
        if let Some(answer) = drain::reply(drain::drain(&reply)) {
            return answer;
        }
    }
}
```

Modify `crates/app/src/views/query_tab/explain.rs`:
```diff
@@ -1,5 +1,5 @@
 //! Explain: asks the session for a statement's plan and shows it in the
-//! Plan view. The reply is polled once per [`POLL`], like a run's events.
+//! Plan view.
 
 use async_channel::Receiver;
 use gpui_kit::Context;
@@ -7,22 +7,18 @@ use quarry_engine::{error::EngineError, plan::Plan};
 
 use super::QueryTab;
 use crate::{
-    model::{drain, plan_tree::PlanState},
-    views::{results, workspace::POLL},
+    model::plan_tree::PlanState,
+    views::{poll::poll_reply, results},
 };
 
 impl QueryTab {
-    /// Shows the Plan view, and the plan once `reply` brings it.
+    /// Shows the Plan view, and the plan once `reply` brings it. A plan
+    /// still pending is dropped, so only the latest Explain is shown.
     pub fn explain(&mut self, reply: Receiver<Result<Plan, EngineError>>, cx: &mut Context<Self>) {
         self.plan = PlanState::Planning;
         self.select_view(results::PLAN, cx);
         self.planning = Some(cx.spawn(async move |this, cx| {
-            let mut reply_seen = None;
-            while reply_seen.is_none() {
-                cx.background_executor().timer(POLL).await;
-                reply_seen = drain::reply(drain::drain(&reply));
-            }
-            let answer = reply_seen.unwrap_or(Err(EngineError::WorkerGone));
+            let answer = poll_reply(reply, cx.background_executor().clone()).await;
             let _ = this.update(cx, |tab, cx| tab.planned(answer, cx));
         }));
     }
```

Modify `crates/app/src/views/workspace/runs.rs`:
```diff
@@ -1,17 +1,16 @@
 //! Submits the active tab's runs and plans to the session, and reads the
 //! schema.
-//!
-//! The schema reply is polled once per [`POLL`] rather than awaited (see
-//! [`crate::model::drain`]).
 
 use gpui_kit::{Context, Window};
 use quarry_engine::splitter;
 
-use super::{POLL, Workspace};
+use super::Workspace;
 use crate::{
-    model::drain,
     shell::actions::{CancelRun, Explain, RunAll, RunStatement},
-    views::query_tab::{QueryTab, RunScope},
+    views::{
+        poll::poll_reply,
+        query_tab::{QueryTab, RunScope},
+    },
 };
 
 impl Workspace {
@@ -65,14 +64,8 @@ impl Workspace {
         let reply = self.session.schema();
         let tree = self.schema.clone();
         self.schema_read = Some(cx.spawn(async move |_, cx| {
-            let mut answer = None;
-            while answer.is_none() {
-                cx.background_executor().timer(POLL).await;
-                answer = drain::reply(drain::drain(&reply));
-            }
-            if let Some(schema) = answer {
-                tree.update(cx, |tree, cx| tree.set(schema, cx));
-            }
+            let schema = poll_reply(reply, cx.background_executor().clone()).await;
+            tree.update(cx, |tree, cx| tree.set(schema, cx));
         }));
     }
 }
```

Modify `crates/app/tests/app/runs.rs`:
```diff
@@ -15,7 +15,11 @@ use quarry::{
         results::{self, statement_tab_id},
     },
 };
-use quarry_engine::{error::EngineError, session::RunEvent};
+use quarry_engine::{
+    error::EngineError,
+    plan::{Plan, PlanNode},
+    session::RunEvent,
+};
 
 use super::support::{drawn, open, phase, run_with, settle, write};
 
@@ -74,7 +78,10 @@ fn a_view_picked_during_a_script_survives_its_end(cx: &mut TestAppContext) {
         });
     });
     assert!(sender.try_send(RunEvent::Done).is_ok());
-    settle(cx, |_, cx| !workspace.read(cx).run_output(cx).is_running());
+    assert!(settle(cx, |_, cx| !workspace
+        .read(cx)
+        .run_output(cx)
+        .is_running()));
     cx.update(|_, cx| assert_eq!(tab.read(cx).view_id(), results::PLAN));
 }
 
@@ -153,19 +160,19 @@ fn a_run_in_another_tab_waits_for_the_running_one(cx: &mut TestAppContext) {
         "SELECT count(*) FROM telemetry a, telemetry b",
     );
     cx.simulate_keystrokes("cmd-enter");
-    settle(cx, |_, cx| {
+    assert!(settle(cx, |_, cx| {
         workspace.read(cx).run_output(cx).phase() == &Phase::Running
-    });
+    }));
     cx.simulate_keystrokes("cmd-t");
     cx.simulate_input("SELECT 1");
     cx.simulate_keystrokes("cmd-enter");
-    settle(cx, |_, _| true);
+    assert!(settle(cx, |_, _| true));
     assert_eq!(phase(&workspace, cx), Phase::Waiting);
     cx.simulate_keystrokes("cmd-{ cmd-.");
     cx.simulate_keystrokes("cmd-}");
-    settle(cx, |_, cx| {
+    assert!(settle(cx, |_, cx| {
         workspace.read(cx).run_output(cx).phase() == &Phase::Finished
-    });
+    }));
     assert_eq!(phase(&workspace, cx), Phase::Finished);
 }
 
@@ -194,13 +201,13 @@ fn cancelling_a_queued_run_ends_it_while_the_other_tab_still_runs(cx: &mut TestA
         "SELECT count(*) FROM telemetry a, telemetry b",
     );
     cx.simulate_keystrokes("cmd-enter");
-    settle(cx, |_, cx| {
+    assert!(settle(cx, |_, cx| {
         workspace.read(cx).run_output(cx).phase() == &Phase::Running
-    });
+    }));
     cx.simulate_keystrokes("cmd-t");
     cx.simulate_input("SELECT 1");
     cx.simulate_keystrokes("cmd-enter");
-    settle(cx, |_, _| true);
+    assert!(settle(cx, |_, _| true));
     assert_eq!(phase(&workspace, cx), Phase::Waiting);
     let ended = record_run_ends(&workspace, cx);
     cx.simulate_keystrokes("cmd-.");
@@ -219,12 +226,12 @@ fn command_e_shows_the_statement_plan(cx: &mut TestAppContext) {
         cx.update(|_, cx| workspace.read(cx).active_tab().read(cx).plan().clone())
     };
     assert_eq!(plan(cx), PlanState::Planning);
-    settle(cx, |_, cx| {
+    assert!(settle(cx, |_, cx| {
         matches!(
             workspace.read(cx).active_tab().read(cx).plan(),
             PlanState::Ready(_)
         )
-    });
+    }));
     let rows = vec![PlanRow {
         depth: 0,
         label: "SEARCH astronauts USING INTEGER PRIMARY KEY (rowid=?)".to_owned(),
@@ -244,12 +251,12 @@ fn a_statement_that_cannot_be_planned_says_why(cx: &mut TestAppContext) {
     let (workspace, cx) = open(cx).expect("the sample database opens");
     write(&workspace, cx, "SELECT * FROM nowhere");
     cx.simulate_keystrokes("cmd-e");
-    settle(cx, |_, cx| {
+    assert!(settle(cx, |_, cx| {
         matches!(
             workspace.read(cx).active_tab().read(cx).plan(),
             PlanState::Failed(_)
         )
-    });
+    }));
     cx.update(|_, cx| {
         let tab = workspace.read(cx).active_tab().read(cx);
         assert_eq!(
@@ -259,3 +266,80 @@ fn a_statement_that_cannot_be_planned_says_why(cx: &mut TestAppContext) {
         assert_eq!(tab.output(cx).phase(), &Phase::Idle, "explain runs nothing");
     });
 }
+
+#[gpui_kit::test]
+fn command_return_from_messages_returns_to_results(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    let tab = cx.update(|_, cx| workspace.read(cx).active_tab().clone());
+    cx.update(|_, cx| tab.update(cx, |tab, cx| tab.select_view(results::MESSAGES, cx)));
+    write(&workspace, cx, "SELECT 1");
+    run_with(&workspace, cx, "cmd-enter");
+    cx.update(|_, cx| assert_eq!(tab.read(cx).view_id(), results::RESULTS));
+}
+
+#[gpui_kit::test]
+fn run_all_with_one_row_returning_statement_shows_no_result_tabs(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    write(
+        &workspace,
+        cx,
+        "CREATE TABLE t (x); SELECT count(*) FROM astronauts;",
+    );
+    run_with(&workspace, cx, "cmd-shift-r");
+    assert_eq!(phase(&workspace, cx), Phase::Finished);
+    assert!(!drawn(cx, statement_tab_id(0)));
+    assert!(!drawn(cx, statement_tab_id(1)));
+}
+
+#[gpui_kit::test]
+fn the_failure_banner_shows_over_messages(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    write(
+        &workspace,
+        cx,
+        "CREATE TABLE a (x); INSERT INTO nowhere VALUES (1);",
+    );
+    run_with(&workspace, cx, "cmd-shift-r");
+    cx.update(|_, cx| {
+        assert_eq!(
+            workspace.read(cx).active_tab().read(cx).view_id(),
+            results::MESSAGES
+        );
+    });
+    assert!(drawn(cx, results::MESSAGES));
+    assert!(drawn(cx, "run-failure"));
+}
+
+fn one_step(label: &str) -> Plan {
+    Plan {
+        nodes: vec![PlanNode {
+            label: label.to_owned(),
+            children: Vec::new(),
+        }],
+    }
+}
+
+#[gpui_kit::test]
+fn a_second_explain_replaces_one_still_pending(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    let tab = cx.update(|_, cx| workspace.read(cx).active_tab().clone());
+    let (first, first_reply) = async_channel::unbounded();
+    let (second, second_reply) = async_channel::unbounded();
+    cx.update(|_, cx| {
+        tab.update(cx, |tab, cx| {
+            tab.explain(first_reply, cx);
+            tab.explain(second_reply, cx);
+        });
+    });
+    assert!(
+        settle(cx, |_, _| first.is_closed()),
+        "the pending Explain is dropped"
+    );
+    assert!(second.try_send(Ok(one_step("second"))).is_ok());
+    assert!(settle(cx, |_, cx| tab.read(cx).plan() != &PlanState::Planning));
+    let rows = vec![PlanRow {
+        depth: 0,
+        label: "second".to_owned(),
+    }];
+    cx.update(|_, cx| assert_eq!(tab.read(cx).plan(), &PlanState::Ready(rows)));
+}
```

Modify `crates/app/tests/app/support.rs`:
```diff
@@ -51,7 +51,7 @@ pub fn open_with(
         let editor = workspace.read(cx).editor(cx);
         editor.update(cx, |state, cx| state.focus(window, cx));
     });
-    settle(cx, |_, _| true);
+    assert!(settle(cx, |_, _| true));
     Ok((workspace, cx))
 }
 
@@ -66,27 +66,30 @@ pub fn write(workspace: &Entity<Workspace>, cx: &mut VisualTestContext, sql: &st
     });
 }
 
-/// Pumps the executor until `done` holds or [`PATIENCE`] runs out; the
-/// session thread delivers events in real time.
-pub fn settle(cx: &mut VisualTestContext, done: impl Fn(&mut Window, &mut App) -> bool) {
+/// Pumps the executor until `done` holds or [`PATIENCE`] runs out, and
+/// says whether `done` held; the session thread delivers events in real
+/// time.
+#[must_use]
+pub fn settle(cx: &mut VisualTestContext, done: impl Fn(&mut Window, &mut App) -> bool) -> bool {
     let deadline = std::time::Instant::now() + PATIENCE;
     while std::time::Instant::now() < deadline {
         cx.executor().advance_clock(POLL);
         cx.run_until_parked();
         if cx.update(|window, cx| done(window, cx)) {
-            return;
+            return true;
         }
         std::thread::sleep(Duration::from_millis(5));
     }
+    false
 }
 
 /// Presses `keys` and waits for the active tab's run to end.
 pub fn run_with(workspace: &Entity<Workspace>, cx: &mut VisualTestContext, keys: &str) {
     cx.simulate_keystrokes(keys);
-    settle(cx, |_, cx| {
+    assert!(settle(cx, |_, cx| {
         let output = workspace.read(cx).run_output(cx);
         !output.is_running() && output.phase() != &Phase::Idle
-    });
+    }));
 }
 
 /// Runs the editor's statement with ⌘↵ and waits for the run to end.
```

Modify `crates/app/tests/app/tabs.rs`:
```diff
@@ -88,6 +88,19 @@ fn closing_the_only_tab_opens_a_fresh_one(cx: &mut TestAppContext) {
     assert_eq!(text(&workspace, cx), "");
 }
 
+#[gpui_kit::test]
+fn command_w_activates_the_right_hand_neighbour(cx: &mut TestAppContext) {
+    install(cx);
+    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
+    let (workspace, cx) = open(cx, store).expect("the sample database opens");
+    cx.simulate_keystrokes("cmd-t cmd-t cmd-{");
+    assert_eq!(titles(&workspace, cx), ["Query 1", "Query 2", "Query 3"]);
+    assert_eq!(active(&workspace, cx), 1);
+    cx.simulate_keystrokes("cmd-w");
+    assert_eq!(titles(&workspace, cx), ["Query 1", "Query 3"]);
+    assert_eq!(active(&workspace, cx), 1);
+}
+
 #[gpui_kit::test]
 fn next_and_previous_switch_tabs_and_wrap(cx: &mut TestAppContext) {
     install(cx);
@@ -124,9 +137,9 @@ fn each_tab_keeps_its_own_results(cx: &mut TestAppContext) {
     let (workspace, cx) = open(cx, store).expect("the sample database opens");
     cx.simulate_keystrokes("cmd-enter");
     let finished = |cx: &mut VisualTestContext| {
-        settle(cx, |_, cx| {
+        assert!(settle(cx, |_, cx| {
             workspace.read(cx).run_output(cx).phase() == &Phase::Finished
-        });
+        }));
     };
     finished(cx);
     cx.simulate_keystrokes("cmd-t");
@@ -153,7 +166,7 @@ fn drafts_come_back_in_a_new_workspace(cx: &mut TestAppContext) {
     cx.simulate_keystrokes("cmd-t");
     cx.simulate_input("SELECT 42");
     cx.executor().advance_clock(AUTOSAVE);
-    settle(cx, |_, _| true);
+    assert!(settle(cx, |_, _| true));
     let saved = store.load().expect("the drafts load");
     assert_eq!(
         (saved.active, saved.tabs[1].text.as_str()),
@@ -183,7 +196,7 @@ fn the_closed_tabs_are_kept_through_a_save(cx: &mut TestAppContext) {
     store.save(&saved).expect("a first save");
     let (_workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
     cx.simulate_keystrokes("cmd-t");
-    settle(cx, |_, _| true);
+    assert!(settle(cx, |_, _| true));
     assert_eq!(store.load().expect("the drafts load").closed, closed);
 }
 
@@ -246,9 +259,9 @@ fn a_store_that_failed_to_load_is_never_saved_over(cx: &mut TestAppContext) {
     let (workspace, cx) = open(cx, store).expect("the sample database opens");
     cx.simulate_input("SELECT 1");
     cx.executor().advance_clock(AUTOSAVE);
-    settle(cx, |_, _| true);
+    assert!(settle(cx, |_, _| true));
     cx.simulate_keystrokes("cmd-t");
-    settle(cx, |_, _| true);
+    assert!(settle(cx, |_, _| true));
     assert!(!saved.load(Ordering::SeqCst));
     cx.update(|_, cx| {
         assert_eq!(
@@ -285,11 +298,11 @@ fn the_warning_clears_after_a_later_save_succeeds(cx: &mut TestAppContext) {
     let (_workspace, cx) = open(cx, store).expect("the sample database opens");
     cx.simulate_input("SELECT 1");
     cx.executor().advance_clock(AUTOSAVE);
-    settle(cx, |_, _| true);
+    assert!(settle(cx, |_, _| true));
     assert!(drawn(cx, "drafts-warning"));
     cx.simulate_input(" + 1");
     cx.executor().advance_clock(AUTOSAVE);
-    settle(cx, |_, _| true);
+    assert!(settle(cx, |_, _| true));
     assert!(!drawn(cx, "drafts-warning"));
 }
 
```

Modify `crates/app/tests/app/workspace.rs`:
```diff
@@ -84,9 +84,15 @@ fn command_period_cancels_a_long_query(cx: &mut TestAppContext) {
         "SELECT count(*) FROM telemetry a, telemetry b",
     );
     cx.simulate_keystrokes("cmd-enter");
-    settle(cx, |_, cx| workspace.read(cx).run_output(cx).is_running());
+    assert!(settle(cx, |_, cx| workspace
+        .read(cx)
+        .run_output(cx)
+        .is_running()));
     cx.simulate_keystrokes("cmd-.");
-    settle(cx, |_, cx| !workspace.read(cx).run_output(cx).is_running());
+    assert!(settle(cx, |_, cx| !workspace
+        .read(cx)
+        .run_output(cx)
+        .is_running()));
     assert_eq!(phase(&workspace, cx), Phase::Failed(EngineError::Cancelled));
 }
 
@@ -99,7 +105,10 @@ fn running_again_during_a_run_replaces_it(cx: &mut TestAppContext) {
         "SELECT count(*) FROM telemetry a, telemetry b",
     );
     cx.simulate_keystrokes("cmd-enter");
-    settle(cx, |_, cx| workspace.read(cx).run_output(cx).is_running());
+    assert!(settle(cx, |_, cx| workspace
+        .read(cx)
+        .run_output(cx)
+        .is_running()));
     write(&workspace, cx, "SELECT 1");
     run(&workspace, cx);
     assert_eq!(phase(&workspace, cx), Phase::Finished);
@@ -114,20 +123,20 @@ fn creating_a_table_adds_it_to_the_schema_tree(cx: &mut TestAppContext) {
     let (workspace, cx) = open(cx).expect("the sample database opens");
     write(&workspace, cx, "CREATE TABLE notes (body TEXT)");
     run(&workspace, cx);
-    settle(cx, |window, cx| {
+    assert!(settle(cx, |window, cx| {
         window.draw(cx).clear(cx);
         window.try_find(table_row_id("notes")).is_some()
-    });
+    }));
     cx.update(|window, _| assert!(window.try_find(table_row_id("notes")).is_some()));
 }
 
 #[gpui_kit::test]
 fn clicking_a_table_expands_it(cx: &mut TestAppContext) {
     let (workspace, cx) = open(cx).expect("the sample database opens");
-    settle(cx, |window, cx| {
+    assert!(settle(cx, |window, cx| {
         window.draw(cx).clear(cx);
         window.try_find(table_row_id("missions")).is_some()
-    });
+    }));
     cx.update(|window, cx| window.click(table_row_id("missions"), cx));
     cx.update(|_, cx| {
         let tree = workspace.read(cx).schema().read(cx);
@@ -156,10 +165,10 @@ fn command_shift_t_cycles_the_theme(cx: &mut TestAppContext) {
 #[gpui_kit::test]
 fn command_one_focuses_the_first_table_and_return_expands_it(cx: &mut TestAppContext) {
     let (workspace, cx) = open(cx).expect("the sample database opens");
-    settle(cx, |window, cx| {
+    assert!(settle(cx, |window, cx| {
         window.draw(cx).clear(cx);
         window.try_find(table_row_id("astronauts")).is_some()
-    });
+    }));
     cx.simulate_keystrokes("cmd-1");
     cx.update(|window, cx| {
         window.draw(cx).clear(cx);
@@ -176,10 +185,10 @@ fn command_one_focuses_the_first_table_and_return_expands_it(cx: &mut TestAppCon
 fn command_two_returns_focus_to_the_editor(cx: &mut TestAppContext) {
     let (workspace, cx) = open(cx).expect("the sample database opens");
     cx.simulate_keystrokes("cmd-1");
-    settle(cx, |window, cx| {
+    assert!(settle(cx, |window, cx| {
         window.draw(cx).clear(cx);
         true
-    });
+    }));
     cx.update(|window, cx| {
         let editor = workspace.read(cx).editor(cx);
         let editor = editor.read(cx);
@@ -212,7 +221,10 @@ fn a_run_whose_channel_closes_without_done_reports_a_lost_worker(cx: &mut TestAp
             tab.follow("SELECT 1".to_owned(), RunScope::Statement, events, cx);
         });
     });
-    settle(cx, |_, cx| !workspace.read(cx).run_output(cx).is_running());
+    assert!(settle(cx, |_, cx| !workspace
+        .read(cx)
+        .run_output(cx)
+        .is_running()));
     assert_eq!(
         phase(&workspace, cx),
         Phase::Failed(EngineError::WorkerGone)
```

- [ ] **Step 2: Run the tests and every gate**

Run: `cargo nextest run -p quarry`
Expected: every test passes.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add crates/app/src/views/mod.rs crates/app/src/views/poll.rs crates/app/src/views/query_tab/explain.rs crates/app/src/views/workspace/runs.rs crates/app/tests/app/runs.rs crates/app/tests/app/support.rs crates/app/tests/app/tabs.rs crates/app/tests/app/workspace.rs
git commit -m "refactor(app): share reply polling and make settle report timeouts"
```

---

### Task 5: Saving in order: `SaveOrder` and the save lock

Every snapshot now takes a generation from `SaveOrder` when it is taken. That covers background saves, the save when the window closes and the save at quit. Each write holds one lock (the `Mutex<SaveOrder>` itself) and is skipped when a newer generation has already been written, so an older snapshot can never land after a newer one. This replaces 1D's chain of save tasks, which the synchronous saves at close and quit didn't wait for. ADR 0010 is updated.

**Files:**
- Modify: `crates/app/src/views/workspace/drafts.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Modify: `crates/app/tests/app/tabs.rs`
- Modify: `docs/adr/0010-tabs-and-drafts.md`

**Interfaces:**
- Consumes: `model::save_order::SaveOrder` (Task 3).
- Produces: `views::workspace::save_in_order(&dyn DraftStore, &Mutex<SaveOrder>, generation: u64, &Drafts) -> Option<Result<(), StoreError>>` (`None` when skipped). The workspace's private `saves: Arc<Mutex<SaveOrder>>` field replaces `saving: Option<Task<()>>`.

- [ ] **Step 1: Write the failing tests**

Modify `crates/app/tests/app/tabs.rs`:
```diff
@@ -2,14 +2,14 @@
 //! drafts across restarts.
 
 use std::sync::{
-    Arc,
+    Arc, Mutex,
     atomic::{AtomicBool, AtomicUsize, Ordering},
 };
 
 use gpui_kit::{Entity, TestAppContext, VisualTestContext, test::TestWindowExt};
 use quarry::{
-    model::statements::Phase,
-    views::workspace::{AUTOSAVE, Workspace, tab_element_id},
+    model::{save_order::SaveOrder, statements::Phase},
+    views::workspace::{AUTOSAVE, Workspace, save_in_order, tab_element_id},
 };
 use quarry_store::{
     drafts::{Draft, DraftStore, Drafts},
@@ -317,3 +317,27 @@ fn closing_the_window_saves_the_last_edits(cx: &mut TestAppContext) {
     let loaded = store.load().expect("the drafts load");
     assert!(loaded.tabs[0].text.starts_with("SELECT 9"));
 }
+
+#[test]
+fn a_delayed_older_snapshot_is_skipped() {
+    let store = SqliteStore::in_memory().expect("an in-memory store");
+    let saves = Mutex::new(SaveOrder::default());
+    let (older, newer) = {
+        let mut order = saves.lock().expect("an unpoisoned lock");
+        (order.issue(), order.issue())
+    };
+    let snapshot = |text: &str| Drafts {
+        tabs: vec![draft(1, "Query 1", text)],
+        active: 0,
+        closed: Vec::new(),
+    };
+    assert_eq!(
+        save_in_order(&store, &saves, newer, &snapshot("newer")),
+        Some(Ok(()))
+    );
+    assert_eq!(
+        save_in_order(&store, &saves, older, &snapshot("older")),
+        None
+    );
+    assert_eq!(store.load(), Ok(snapshot("newer")));
+}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry a_delayed_older_snapshot_is_skipped`
Expected: FAIL to compile: no `save_in_order` in `quarry::views::workspace`.

- [ ] **Step 3: Implement**

Modify `crates/app/src/views/workspace/drafts.rs`:
```diff
@@ -1,10 +1,13 @@
 //! Saving the tabs' drafts: a second after the last edit, at once when the
 //! tabs change, when the window closes and when Quarry quits.
 //!
-//! Writes run on the background executor, one after another, so a later
-//! snapshot never lands before an earlier one.
+//! Every snapshot gets a generation when it is taken, and every write holds
+//! one lock, so an older snapshot never lands after a newer one.
 
-use std::{sync::Arc, time::Duration};
+use std::{
+    sync::{Arc, Mutex, PoisonError},
+    time::Duration,
+};
 
 use gpui_kit::{App, AppContext, Context, WindowId};
 use quarry_store::{
@@ -13,6 +16,7 @@ use quarry_store::{
 };
 
 use super::Workspace;
+use crate::model::save_order::SaveOrder;
 
 /// How long after the last edit the drafts are saved.
 pub const AUTOSAVE: Duration = Duration::from_secs(1);
@@ -91,26 +95,35 @@ impl Workspace {
         }));
     }
 
-    /// Saves a snapshot now, after any save still being written.
+    /// A snapshot and its generation.
+    fn numbered_snapshot(&self, cx: &App) -> (u64, Drafts) {
+        let generation = lock(&self.saves).issue();
+        (generation, self.snapshot(cx))
+    }
+
+    /// Saves a snapshot now, on the background executor.
     pub(super) fn save_now(&mut self, cx: &mut Context<Self>) {
         self.autosave = None;
-        let drafts = self.snapshot(cx);
+        let (generation, drafts) = self.numbered_snapshot(cx);
         let store = Arc::clone(&self.store);
-        let previous = self.saving.take();
-        self.saving = Some(cx.spawn(async move |this, cx| {
-            if let Some(previous) = previous {
-                previous.await;
-            }
-            let saved = cx
-                .background_spawn(async move { store.save(&drafts) })
+        let saves = Arc::clone(&self.saves);
+        cx.spawn(async move |this, cx| {
+            let outcome = cx
+                .background_spawn(
+                    async move { save_in_order(&*store, &saves, generation, &drafts) },
+                )
                 .await;
-            let _ = this.update(cx, |workspace, cx| workspace.saved(saved, cx));
-        }));
+            if let Some(outcome) = outcome {
+                let _ = this.update(cx, |workspace, cx| workspace.saved(outcome, cx));
+            }
+        })
+        .detach();
     }
 
-    /// Saves synchronously, for when Quarry is quitting.
+    /// Saves synchronously, for when the window closes or Quarry quits.
     pub(super) fn save_before_quit(&self, cx: &App) {
-        let _ = self.store.save(&self.snapshot(cx));
+        let (generation, drafts) = self.numbered_snapshot(cx);
+        let _ = save_in_order(&*self.store, &self.saves, generation, &drafts);
     }
 
     fn saved(&mut self, saved: Result<(), StoreError>, cx: &mut Context<Self>) {
@@ -118,3 +131,21 @@ impl Workspace {
         cx.notify();
     }
 }
+
+/// Writes `drafts`, snapshot `generation`, to `store` while holding the save
+/// lock `saves`. Returns `None`, writing nothing, when a newer snapshot has
+/// already been written.
+pub fn save_in_order(
+    store: &dyn DraftStore,
+    saves: &Mutex<SaveOrder>,
+    generation: u64,
+    drafts: &Drafts,
+) -> Option<Result<(), StoreError>> {
+    let mut order = lock(saves);
+    order.commit(generation).then(|| store.save(drafts))
+}
+
+/// The save lock; a panic while it was held leaves nothing to undo.
+fn lock(saves: &Mutex<SaveOrder>) -> std::sync::MutexGuard<'_, SaveOrder> {
+    saves.lock().unwrap_or_else(PoisonError::into_inner)
+}
```

Modify `crates/app/src/views/workspace/mod.rs`:
```diff
@@ -10,7 +10,11 @@ mod runs;
 mod tab_strip;
 mod tabs;
 
-use std::{collections::BTreeMap, sync::Arc, time::Duration};
+use std::{
+    collections::BTreeMap,
+    sync::{Arc, Mutex},
+    time::Duration,
+};
 
 use gpui_kit::{
     App, AppContext, Context, Entity, FocusHandle, Focusable, Subscription, Task, Window,
@@ -21,12 +25,13 @@ use quarry_store::drafts::{Draft, DraftStore, Drafts};
 use quarry_ui_kit::Aurora;
 
 pub use self::{
-    drafts::{AUTOSAVE, SavedTabs},
+    drafts::{AUTOSAVE, SavedTabs, save_in_order},
     tab_strip::tab_element_id,
 };
 use super::{editor, query_tab::QueryTab, schema::SchemaTree};
 use crate::{
     model::{
+        save_order::SaveOrder,
         statements::RunOutput,
         tabs::{Tab, TabId, TabList},
     },
@@ -52,7 +57,7 @@ pub struct Workspace {
     open_problem: Option<String>,
     save_problem: Option<String>,
     autosave: Option<Task<()>>,
-    saving: Option<Task<()>>,
+    saves: Arc<Mutex<SaveOrder>>,
     schema: Entity<SchemaTree>,
     sidebar_open: bool,
     focus: FocusHandle,
@@ -82,7 +87,7 @@ impl Workspace {
             open_problem: problem,
             save_problem: None,
             autosave: None,
-            saving: None,
+            saves: Arc::default(),
             schema: cx.new(|_| SchemaTree::default()),
             sidebar_open: true,
             focus: cx.focus_handle(),
```

Modify `docs/adr/0010-tabs-and-drafts.md`:
```diff
@@ -13,7 +13,7 @@ Phase 1D turns the single editor of Phase 1C (ADR 0009) into query tabs whose dr
 - **Result views are a registry.** `ResultView` and `ResultViewRegistry` hold Results, Messages and Plan; a new view is a module and one line. A segmented control switches between them.
 - **Plans come from the driver.** `Driver::plan` defaults to `PlanUnsupported`. SQLite builds a tree from `EXPLAIN QUERY PLAN`, and `Session::plan` runs it on the session thread after any queued run, like `schema()`. It plans only the first statement of its text, so ⌘E on a multi-statement selection works.
 - **Drafts live in `quarry-store`.** `DraftStore` loads and saves `Drafts` (tabs and the active index); `SqliteStore` keeps them in `~/Library/Application Support/Quarry/quarry.db`, migrated with `PRAGMA user_version`. A file from a newer Quarry is refused, never changed. If the store can't be opened, `shell::store` falls back to a store that keeps nothing, and the status bar says drafts aren't being saved. If it opens but its drafts can't be read, the workspace falls back the same way, so a save never writes a fresh tab over drafts that are still on disk.
-- **When drafts are saved.** One second after the last edit (each edit replaces the pending timer), at once when tabs open, close or switch, and synchronously when the window closes or Quarry quits. Background saves run one after another, so they never land out of order with each other. The synchronous saves on close and quit do not wait for a background save that is already running.
+- **When drafts are saved.** One second after the last edit (each edit replaces the pending timer), at once when tabs open, close or switch, and synchronously when the window closes or Quarry quits. Every snapshot gets a rising generation from `SaveOrder` when it is taken, and every write, background or synchronous, holds one save lock. A write is skipped when a newer generation has already been written, so an older snapshot never lands after a newer one, including a background save that finishes after the save at close or quit. (Phase 1E; until then the synchronous saves did not wait for a background save already running.)
 - **Streaming throughput.** The session buffers 128 batches of 500 rows per run instead of 16. With the UI polling every 33 ms, the 200,000-row sample table streams in about 0.16 s instead of about 1 s; the engine's own time drops from about 0.93 s to 0.12 s, because it no longer waits on the reader.
 - **App lifecycle.** The shell follows the system appearance and quits when the last window closes.
 
```

- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry`
Expected: every test passes, including `a_delayed_older_snapshot_is_skipped` and the 1D draft tests (`drafts_come_back_in_a_new_workspace`, `the_warning_clears_after_a_later_save_succeeds`, `closing_the_window_saves_the_last_edits`).

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/app/src/views/workspace/drafts.rs crates/app/src/views/workspace/mod.rs crates/app/tests/app/tabs.rs docs/adr/0010-tabs-and-drafts.md
git commit -m "fix(app): save drafts in order through one lock"
```

---

### Task 6: Copying: `QueryTab::copy`, the status-bar note, the Edit menu and ⌘C

`QueryTab::copy(format, cx)` reads the grid's selection and the shown statement's output and builds a `Copied` with `model::copy`. It formats the text on the background executor, writes the clipboard on the main thread, and emits `QueryTabEvent::Copied(note)`. The workspace shows the note in the status bar for `NOTE_TIME` (3 s), and a newer note replaces the older one and its timer. ⌘C is bound in the grid's `DataTable` context only, so the editor keeps its own ⌘C. A new Edit menu holds Copy as TSV, CSV and JSON. The bindings test now allows the two copy commands that have no key; Task 7 rewrites it around the command table.

**Files:**
- Modify: `crates/app/src/shell/actions.rs`
- Create: `crates/app/src/views/query_tab/copy.rs`
- Modify: `crates/app/src/views/query_tab/mod.rs`
- Create: `crates/app/src/views/workspace/copy.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Modify: `crates/app/src/views/workspace/tabs.rs`
- Create: `crates/app/tests/app/copy.rs`
- Modify: `crates/app/tests/app/main.rs`
- Modify: `crates/app/tests/shell.rs`

**Interfaces:**
- Consumes: `ExporterRegistry` (Task 1), `model::copy` (Task 3), gpui-component's `TableState::selection() -> TableSelection`.
- Produces:
  - Actions `CopyTsv`, `CopyCsv` and `CopyJson`; `shell::actions::GRID = "DataTable"`.
  - `QueryTab::copy(&mut self, format: &'static str, cx)`.
  - `QueryTabEvent::Copied(String)`. `QueryTabEvent` is now `Clone`, not `Copy`.
  - `views::workspace::NOTE_TIME`, and `Workspace::note(&self) -> Option<&str>`.
  - The note is drawn with id `copy-note`.

- [ ] **Step 1: Write the failing tests**

Create `crates/app/tests/app/copy.rs`:
```rust
//! Copying results: ⌘C in the grid, Edit → Copy as …, and the note in the
//! status bar.

use gpui_kit::{Entity, Focusable, TestAppContext, VisualTestContext};
use quarry::{
    shell::actions::{CopyCsv, CopyJson, CopyTsv},
    views::workspace::{NOTE_TIME, Workspace},
};

use super::support::{drawn, open, run, settle, write};

const TWO_ROWS: &str = "SELECT 1 AS a, 'x' AS b UNION ALL SELECT 2, 'y'";

fn clipboard(cx: &mut VisualTestContext) -> Option<String> {
    cx.read_from_clipboard().and_then(|item| item.text())
}

fn note(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|_, cx| workspace.read(cx).note().map(str::to_owned))
}

/// Runs [`TWO_ROWS`] and focuses the grid.
fn two_rows_in_the_grid(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) {
    write(workspace, cx, TWO_ROWS);
    run(workspace, cx);
    cx.update(|window, cx| {
        let grid = workspace.read(cx).active_tab().read(cx).results().clone();
        let handle = grid.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
    });
}

/// Waits for the copy's note.
fn copied(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Option<String> {
    let shown = settle(cx, |_, cx| workspace.read(cx).note().is_some());
    shown.then(|| note(workspace, cx)).flatten()
}

#[gpui_kit::test]
fn command_c_in_the_grid_copies_every_row_as_tsv(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    two_rows_in_the_grid(&workspace, cx);
    cx.simulate_keystrokes("cmd-c");
    assert_eq!(
        copied(&workspace, cx).as_deref(),
        Some("Copied 2 rows as TSV")
    );
    assert_eq!(clipboard(cx).as_deref(), Some("a\tb\n1\tx\n2\ty\n"));
    assert!(drawn(cx, "copy-note"));
}

#[gpui_kit::test]
fn copy_as_csv_copies_the_selected_row(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    two_rows_in_the_grid(&workspace, cx);
    cx.update(|window, cx| {
        let grid = workspace.read(cx).active_tab().read(cx).results().clone();
        grid.update(cx, |table, cx| table.set_selected_row(1, cx));
        window.dispatch_action(Box::new(CopyCsv), cx);
    });
    assert_eq!(
        copied(&workspace, cx).as_deref(),
        Some("Copied 1 row as CSV")
    );
    assert_eq!(clipboard(cx).as_deref(), Some("a,b\r\n2,y\r\n"));
}

#[gpui_kit::test]
fn copy_as_json_copies_the_selected_cell_from_the_editor(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    two_rows_in_the_grid(&workspace, cx);
    cx.update(|window, cx| {
        let grid = workspace.read(cx).active_tab().read(cx).results().clone();
        grid.update(cx, |table, cx| table.set_selected_cell(0, 1, cx));
        let editor = workspace.read(cx).editor(cx);
        editor.update(cx, |state, cx| state.focus(window, cx));
        window.dispatch_action(Box::new(CopyJson), cx);
    });
    assert_eq!(
        copied(&workspace, cx).as_deref(),
        Some("Copied 1 value as JSON")
    );
    assert_eq!(clipboard(cx).as_deref(), Some("\"x\""));
}

#[gpui_kit::test]
fn copy_as_json_copies_a_column(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    two_rows_in_the_grid(&workspace, cx);
    cx.update(|window, cx| {
        let grid = workspace.read(cx).active_tab().read(cx).results().clone();
        grid.update(cx, |table, cx| table.set_selected_col(0, cx));
        window.dispatch_action(Box::new(CopyJson), cx);
    });
    assert_eq!(
        copied(&workspace, cx).as_deref(),
        Some("Copied 2 rows as JSON")
    );
    assert_eq!(
        clipboard(cx).as_deref(),
        Some("[\n  {\n    \"a\": 1\n  },\n  {\n    \"a\": 2\n  }\n]")
    );
}

#[gpui_kit::test]
fn nothing_to_copy_before_a_run_or_without_rows(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.update(|window, cx| window.dispatch_action(Box::new(CopyTsv), cx));
    assert_eq!(copied(&workspace, cx).as_deref(), Some("Nothing to copy"));
    assert_eq!(clipboard(cx), None);
    write(&workspace, cx, "SELECT 1 WHERE 0");
    run(&workspace, cx);
    cx.update(|window, cx| window.dispatch_action(Box::new(CopyTsv), cx));
    assert_eq!(copied(&workspace, cx).as_deref(), Some("Nothing to copy"));
    assert_eq!(clipboard(cx), None);
}

#[gpui_kit::test]
fn the_note_goes_after_three_seconds(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.update(|window, cx| window.dispatch_action(Box::new(CopyTsv), cx));
    assert!(copied(&workspace, cx).is_some());
    cx.executor().advance_clock(NOTE_TIME);
    cx.run_until_parked();
    assert_eq!(note(&workspace, cx), None);
    assert!(!drawn(cx, "copy-note"));
}

#[gpui_kit::test]
fn a_newer_note_replaces_an_older_one_and_its_timer(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    two_rows_in_the_grid(&workspace, cx);
    cx.update(|window, cx| window.dispatch_action(Box::new(CopyCsv), cx));
    assert_eq!(
        copied(&workspace, cx).as_deref(),
        Some("Copied 2 rows as CSV")
    );
    cx.executor().advance_clock(NOTE_TIME / 2);
    cx.simulate_keystrokes("cmd-c");
    assert!(settle(cx, |_, cx| {
        workspace.read(cx).note() == Some("Copied 2 rows as TSV")
    }));
    cx.executor().advance_clock(NOTE_TIME / 2);
    cx.run_until_parked();
    assert_eq!(
        note(&workspace, cx).as_deref(),
        Some("Copied 2 rows as TSV")
    );
}

#[gpui_kit::test]
fn command_c_in_the_editor_copies_the_editor_text(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, TWO_ROWS);
    run(&workspace, cx);
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor(cx);
        editor.update(cx, |state, cx| {
            state.focus(window, cx);
            state.set_selected_range(0..6, cx);
        });
    });
    cx.simulate_keystrokes("cmd-c");
    cx.run_until_parked();
    assert_eq!(clipboard(cx).as_deref(), Some("SELECT"));
    assert_eq!(note(&workspace, cx), None);
}
```

Modify `crates/app/tests/app/main.rs`:
```diff
@@ -2,6 +2,7 @@
 //!
 //! One test binary, so the helpers in [`support`] are shared.
 
+mod copy;
 mod runs;
 mod support;
 mod tabs;
```

Modify `crates/app/tests/shell.rs`:
```diff
@@ -49,9 +49,13 @@ fn every_menu_command_has_a_key_binding() {
             _ => None,
         })
         .collect();
-    assert_eq!(commands.len(), 13);
+    assert_eq!(commands.len(), 16);
+    let unbound = ["quarry::CopyCsv", "quarry::CopyJson"];
     for command in commands {
-        assert!(bound.contains(&command), "{command} has no key binding");
+        assert!(
+            bound.contains(&command) != unbound.contains(&command),
+            "{command} has no key binding"
+        );
     }
 }
 
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry copy`
Expected: FAIL to compile: no `CopyTsv` in `quarry::shell::actions`, no `NOTE_TIME` in `quarry::views::workspace`.

- [ ] **Step 3: Implement**

Modify `crates/app/src/shell/actions.rs`:
```diff
@@ -32,6 +32,12 @@ actions!(
         FocusEditor,
         /// Cycles the theme: Dark, Light, System.
         CycleTheme,
+        /// Copies the grid's selection, or the whole result, as TSV.
+        CopyTsv,
+        /// Copies the grid's selection, or the whole result, as CSV.
+        CopyCsv,
+        /// Copies the grid's selection, or the whole result, as JSON.
+        CopyJson,
         /// Quits Quarry.
         Quit,
     ]
@@ -44,6 +50,10 @@ pub const WORKSPACE: &str = "Workspace";
 /// over the editor's built-in ⌘↵ (newline) and ⌘. (code actions).
 pub const WORKSPACE_EDITOR: &str = "Workspace > Input";
 
+/// The results grid's key context. ⌘C there copies as TSV; the editor
+/// keeps its own ⌘C.
+pub const GRID: &str = "DataTable";
+
 /// Every key binding Quarry adds.
 pub fn key_bindings() -> Vec<KeyBinding> {
     vec![
@@ -61,6 +71,7 @@ pub fn key_bindings() -> Vec<KeyBinding> {
         KeyBinding::new("cmd-1", FocusSidebar, None),
         KeyBinding::new("cmd-2", FocusEditor, None),
         KeyBinding::new("cmd-shift-t", CycleTheme, None),
+        KeyBinding::new("cmd-c", CopyTsv, Some(GRID)),
         KeyBinding::new("cmd-q", Quit, None),
     ]
 }
@@ -73,6 +84,11 @@ pub fn menus() -> Vec<Menu> {
             MenuItem::action("New Tab", NewTab),
             MenuItem::action("Close Tab", CloseTab),
         ]),
+        Menu::new("Edit").items([
+            MenuItem::action("Copy as TSV", CopyTsv),
+            MenuItem::action("Copy as CSV", CopyCsv),
+            MenuItem::action("Copy as JSON", CopyJson),
+        ]),
         Menu::new("Query").items([
             MenuItem::action("Run Statement", RunStatement),
             MenuItem::action("Run All", RunAll),
```

Create `crates/app/src/views/query_tab/copy.rs`:
```rust
//! Copying the grid: its selection, or the whole shown result, as text.
//!
//! The rows are formatted on the background executor and the clipboard is
//! written on the main thread.

use gpui_kit::{AppContext, ClipboardItem, Context, component::table::TableSelection};
use quarry_engine::export::{Exporter, ExporterRegistry};

use super::{QueryTab, QueryTabEvent};
use crate::model::copy::{self, Copied, Selection};

/// The model's view of the grid's selection.
fn selection(table: TableSelection) -> Selection {
    match table {
        TableSelection::None => Selection::None,
        TableSelection::Row(row) => Selection::Row(row),
        TableSelection::Column(column) => Selection::Column(column),
        TableSelection::Cell(row, column) => Selection::Cell(row, column),
    }
}

impl QueryTab {
    /// Copies the grid's selection, or every kept row of the shown result,
    /// as the format named `format` (an [`ExporterRegistry`] id), and emits
    /// [`QueryTabEvent::Copied`] with the note to show. An unknown format
    /// copies nothing.
    pub fn copy(&mut self, format: &'static str, cx: &mut Context<Self>) {
        let registry = ExporterRegistry::standard();
        let Some(title) = registry.get(format).map(Exporter::title) else {
            return;
        };
        let copied = self.copied(cx);
        let note = copy::note(copied.as_ref(), title);
        let Some(copied) = copied else {
            cx.emit(QueryTabEvent::Copied(note));
            return;
        };
        self.copying = Some(cx.spawn(async move |this, cx| {
            let text = cx
                .background_spawn(async move {
                    let registry = ExporterRegistry::standard();
                    registry.get(format).map(|exporter| copied.text(exporter))
                })
                .await;
            let _ = this.update(cx, |_, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(text.unwrap_or_default()));
                cx.emit(QueryTabEvent::Copied(note));
            });
        }));
    }

    /// What a copy would take from the shown statement, if anything.
    fn copied(&self, cx: &Context<Self>) -> Option<Copied> {
        let table = self.results.read(cx);
        let delegate = table.delegate();
        let statement = delegate.output().statement(delegate.shown())?;
        copy::copied(
            statement.columns(),
            statement.rows(),
            statement.received(),
            selection(table.selection()),
        )
    }
}
```

Modify `crates/app/src/views/query_tab/mod.rs`:
```diff
@@ -4,6 +4,7 @@
 //! runs; a tab follows its run's events (see [`pump`]) and tells the
 //! workspace when the run ends.
 
+mod copy;
 mod explain;
 mod layout;
 mod pump;
@@ -29,12 +30,14 @@ use crate::model::{
 };
 
 /// What a tab tells the workspace.
-#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+#[derive(Debug, Clone, PartialEq, Eq)]
 pub enum QueryTabEvent {
     /// The tab's run ended; the schema may have changed.
     RunEnded,
     /// The user changed the editor's text.
     Edited,
+    /// A copy finished; the note says what it copied.
+    Copied(String),
 }
 
 /// What a run was asked to run.
@@ -57,6 +60,7 @@ pub struct QueryTab {
     pump: Option<Task<()>>,
     plan: PlanState,
     planning: Option<Task<()>>,
+    copying: Option<Task<()>>,
     _edits: Subscription,
 }
 
@@ -85,6 +89,7 @@ impl QueryTab {
             pump: None,
             plan: PlanState::Idle,
             planning: None,
+            copying: None,
             _edits: edits,
         }
     }
```

Create `crates/app/src/views/workspace/copy.rs`:
```rust
//! Copying the active tab's result, and the note in the status bar that
//! says what was copied.

use std::time::Duration;

use gpui_kit::{Context, Window};

use super::Workspace;
use crate::shell::actions::{CopyCsv, CopyJson, CopyTsv};

/// How long the status bar shows a copy's note.
pub const NOTE_TIME: Duration = Duration::from_secs(3);

impl Workspace {
    pub(super) fn copy_tsv(&mut self, _: &CopyTsv, _: &mut Window, cx: &mut Context<Self>) {
        self.copy("tsv", cx);
    }

    pub(super) fn copy_csv(&mut self, _: &CopyCsv, _: &mut Window, cx: &mut Context<Self>) {
        self.copy("csv", cx);
    }

    pub(super) fn copy_json(&mut self, _: &CopyJson, _: &mut Window, cx: &mut Context<Self>) {
        self.copy("json", cx);
    }

    /// Copies the active tab's result as `format`.
    fn copy(&mut self, format: &'static str, cx: &mut Context<Self>) {
        self.active_tab()
            .clone()
            .update(cx, |tab, cx| tab.copy(format, cx));
    }

    /// Shows `note` in the status bar for [`NOTE_TIME`], replacing any
    /// note still showing.
    pub(super) fn show_note(&mut self, note: String, cx: &mut Context<Self>) {
        self.note = Some(note);
        self.note_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(NOTE_TIME).await;
            let _ = this.update(cx, |workspace, cx| {
                workspace.note = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// The note the status bar shows after a copy, while it shows one.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}
```

Modify `crates/app/src/views/workspace/layout.rs`:
```diff
@@ -44,6 +44,9 @@ impl Render for Workspace {
             .on_action(cx.listener(Self::focus_sidebar))
             .on_action(cx.listener(Self::focus_editor))
             .on_action(cx.listener(Self::cycle_theme))
+            .on_action(cx.listener(Self::copy_tsv))
+            .on_action(cx.listener(Self::copy_csv))
+            .on_action(cx.listener(Self::copy_json))
             .size_full()
             .flex()
             .bg(window_background(&colors))
@@ -107,6 +110,9 @@ impl Workspace {
         if let Some(problem) = self.drafts_problem() {
             bar = bar.left(drafts_warning(problem));
         }
+        if let Some(note) = self.note() {
+            bar = bar.left(copy_note(note));
+        }
         div()
             .id("status")
             .child(bar.right(format!("{summary} · {place}")))
@@ -124,3 +130,12 @@ fn drafts_warning(problem: &str) -> AnyElement {
         .test_support()
         .into_any_element()
 }
+
+/// What the last copy took, as in "Copied 12 rows as CSV".
+fn copy_note(note: &str) -> AnyElement {
+    div()
+        .id("copy-note")
+        .child(note.to_owned())
+        .test_support()
+        .into_any_element()
+}
```

Modify `crates/app/src/views/workspace/mod.rs`:
```diff
@@ -4,6 +4,7 @@
 //! [`tabs`] opens, closes and switches tabs, [`drafts`] saves them,
 //! [`runs`] submits runs and reads the schema, and [`layout`] draws it.
 
+mod copy;
 mod drafts;
 mod layout;
 mod runs;
@@ -25,6 +26,7 @@ use quarry_store::drafts::{Draft, DraftStore, Drafts};
 use quarry_ui_kit::Aurora;
 
 pub use self::{
+    copy::NOTE_TIME,
     drafts::{AUTOSAVE, SavedTabs, save_in_order},
     tab_strip::tab_element_id,
 };
@@ -63,6 +65,8 @@ pub struct Workspace {
     focus: FocusHandle,
     sidebar_focus: FocusHandle,
     schema_read: Option<Task<()>>,
+    note: Option<String>,
+    note_timer: Option<Task<()>>,
     _quit: Subscription,
 }
 
@@ -93,6 +97,8 @@ impl Workspace {
             focus: cx.focus_handle(),
             sidebar_focus: cx.focus_handle().tab_index(0).tab_stop(false),
             schema_read: None,
+            note: None,
+            note_timer: None,
             _quit: cx.on_app_quit(|workspace, cx| {
                 workspace.save_before_quit(cx);
                 async {}
```

Modify `crates/app/src/views/workspace/tabs.rs`:
```diff
@@ -22,6 +22,7 @@ impl Workspace {
         cx.subscribe(&view, |workspace, _, event, cx| match event {
             QueryTabEvent::RunEnded => workspace.read_schema(cx),
             QueryTabEvent::Edited => workspace.schedule_save(cx),
+            QueryTabEvent::Copied(note) => workspace.show_note(note.clone(), cx),
         })
         .detach();
         self.views.insert(id, view);
```

- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry copy shell`
Expected: every test passes, including `command_c_in_the_grid_copies_every_row_as_tsv`, `copy_as_csv_copies_the_selected_row`, `copy_as_json_copies_the_selected_cell_from_the_editor`, `copy_as_json_copies_a_column`, `nothing_to_copy_before_a_run_or_without_rows`, `the_note_goes_after_three_seconds`, `a_newer_note_replaces_an_older_one_and_its_timer` and `command_c_in_the_editor_copies_the_editor_text`.

Run: `cargo nextest run -p quarry`
Expected: every test passes.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/app/src/shell/actions.rs crates/app/src/views/query_tab/copy.rs crates/app/src/views/query_tab/mod.rs crates/app/src/views/workspace/copy.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/mod.rs crates/app/src/views/workspace/tabs.rs crates/app/tests/app/copy.rs crates/app/tests/app/main.rs crates/app/tests/shell.rs
git commit -m "feat(app): copy results as TSV, CSV and JSON"
```

---

### Task 7: The palette: `PaletteSource`, the registry, the dialog, Open tabs and Commands, ⌘K and the command table

This task builds the palette and its first two sources:
- **`PaletteSource` and `PaletteSourceRegistry`:** the registry starts with Open tabs and Commands; Task 8 adds the other two in between.
- **The dialog:** it wraps gpui-component's `Command` with `filterable(false)`. Each query re-runs `model::palette::filter` over the items collected when the palette opened. Confirming closes the dialog first, then asks the item's source to act, so focus is back in the workspace when the source dispatches a command or activates a tab.
- **Opening and closing:** the dialog opens through gpui-kit's `Root` dialog layer. Esc and a click outside close it, `Root` restores focus, and ⌘K closes it again through the `Palette` key context.
- **The command table:** `shell::actions::commands()` becomes the single table that makes the menus, feeds the bindings test and lists the palette's commands. Quit, in the app menu, is left out of the palette.
- **Test support:** test windows now wrap the workspace in `Root`, as the real window does.

**Files:**
- Modify: `crates/app/src/shell/actions.rs`
- Modify: `crates/app/src/views/mod.rs`
- Create: `crates/app/src/views/palette/commands.rs`
- Create: `crates/app/src/views/palette/dialog.rs`
- Create: `crates/app/src/views/palette/mod.rs`
- Create: `crates/app/src/views/palette/tabs.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Create: `crates/app/src/views/workspace/palette.rs`
- Modify: `crates/app/tests/app/main.rs`
- Create: `crates/app/tests/app/palette.rs`
- Modify: `crates/app/tests/app/support.rs`
- Modify: `crates/app/tests/shell.rs`

**Interfaces:**
- Consumes: `model::palette` (Task 3), `Workspace::{tabs, activate_tab}`, gpui-component's `command::{Command, CommandGroup, CommandItem, CommandState}`, `WindowExt::{open_dialog, close_dialog}`, and `kbd::Kbd::format`.
- Produces:
  - **Shell:**
    - The `OpenPalette` action, and `shell::actions::{PALETTE = "Palette", APP_MENU = "Quarry"}`.
    - `Command { label: &'static str, action: fn() -> Box<dyn Action>, menu: &'static str }` and `commands() -> Vec<Command>` (17 rows).
    - `menus()`, now built from the table.
  - **Palette views:**
    - `views::palette::{PaletteSource { id, title, items(&self, &Workspace, &App) -> Vec<PaletteItem>, pick(&self, &str, &mut Workspace, &mut Window, &mut Context<Workspace>) }, PaletteSourceRegistry::{standard, sources, get, items}}`.
    - `views::palette::dialog::{Palette::{groups(&self) -> Vec<PaletteGroup>, state(&self) -> &Entity<CommandState>}, PaletteGroup { title: &'static str, items: Vec<PaletteItem> }, open(...), PALETTE_WIDTH, PLACEHOLDER}`.
    - `Workspace::palette(&self) -> Option<Entity<Palette>>`, which is `Some` while the palette is open.
  - **Element ids:** `palette` and `palette-empty`.

- [ ] **Step 1: Write the failing tests**

Modify `crates/app/tests/app/main.rs`:
```diff
@@ -3,6 +3,7 @@
 //! One test binary, so the helpers in [`support`] are shared.
 
 mod copy;
+mod palette;
 mod runs;
 mod support;
 mod tabs;
```

Create `crates/app/tests/app/palette.rs`:
```rust
//! The ⌘K palette: opening and closing it, filtering, and picking from
//! each source.

use gpui_kit::{Entity, Focusable, TestAppContext, VisualTestContext, component::WindowExt};
use quarry::{
    shell::actions::CloseTab,
    views::{palette::dialog::PaletteGroup, workspace::Workspace},
};

use super::support::{drawn, open, phase, settle};

/// Whether the editor of the active tab has focus.
fn editor_focused(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> bool {
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor(cx);
        editor
            .read(cx)
            .focus_handle(cx)
            .contains_focused(window, cx)
    })
}

fn is_open(cx: &mut VisualTestContext) -> bool {
    cx.update(WindowExt::has_active_dialog)
}

/// The palette's groups as (heading, labels).
fn groups(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Vec<(String, Vec<String>)> {
    cx.update(|_, cx| {
        let palette = workspace.read(cx).palette();
        let groups: Vec<PaletteGroup> = palette.map(|p| p.read(cx).groups()).unwrap_or_default();
        groups
            .into_iter()
            .map(|group| {
                let labels = group.items.into_iter().map(|item| item.label).collect();
                (group.title.to_owned(), labels)
            })
            .collect()
    })
}

/// Opens the palette with ⌘K and types `query`.
fn search(cx: &mut VisualTestContext, query: &str) {
    cx.simulate_keystrokes("cmd-k");
    cx.run_until_parked();
    if !query.is_empty() {
        cx.simulate_input(query);
        cx.run_until_parked();
    }
}

#[gpui_kit::test]
fn command_k_opens_the_palette_from_the_editor_and_escape_closes_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    assert!(editor_focused(&workspace, cx));
    search(cx, "");
    assert!(is_open(cx));
    assert!(drawn(cx, "palette"));
    assert!(!editor_focused(&workspace, cx));
    cx.simulate_keystrokes("escape");
    assert!(settle(cx, |window, cx| !window.has_active_dialog(cx)));
    assert!(editor_focused(&workspace, cx));
}

#[gpui_kit::test]
fn command_k_again_closes_the_palette(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    search(cx, "");
    cx.simulate_keystrokes("cmd-k");
    assert!(settle(cx, |window, cx| !window.has_active_dialog(cx)));
    assert!(editor_focused(&workspace, cx));
}

#[gpui_kit::test]
fn an_empty_query_lists_open_tabs_then_commands(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    search(cx, "");
    let groups = groups(&workspace, cx);
    let headings: Vec<&str> = groups.iter().map(|(title, _)| title.as_str()).collect();
    assert_eq!(headings, ["Open tabs", "Commands"]);
    assert_eq!(groups[0].1, ["Query 1", "Query 2"]);
    assert_eq!(groups[1].1[..3], ["New Tab", "Close Tab", "Copy as TSV"]);
    assert!(!groups[1].1.contains(&"Quit Quarry".to_owned()));
    let current = cx.update(|_, cx| {
        let palette = workspace.read(cx).palette();
        palette.map(|p| p.read(cx).groups()[0].items[1].detail.clone())
    });
    assert_eq!(current.as_deref(), Some("current"));
}

#[gpui_kit::test]
fn typing_hides_groups_without_matches_and_says_when_nothing_matches(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    search(cx, "sidebar");
    let found = groups(&workspace, cx);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1, ["Toggle Sidebar", "Focus Sidebar"]);
    assert!(!drawn(cx, "palette-empty"));
    cx.simulate_input("zzz");
    cx.run_until_parked();
    assert!(groups(&workspace, cx).is_empty());
    assert!(drawn(cx, "palette-empty"));
}

#[gpui_kit::test]
fn picking_an_open_tab_activates_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    search(cx, "query 1");
    cx.simulate_keystrokes("enter");
    assert!(settle(cx, |window, cx| !window.has_active_dialog(cx)));
    cx.update(|_, cx| assert_eq!(workspace.read(cx).tabs().active(), 0));
    assert!(editor_focused(&workspace, cx));
}

#[gpui_kit::test]
fn picking_a_command_runs_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    search(cx, "run all");
    cx.simulate_keystrokes("enter");
    assert!(settle(cx, |_, cx| {
        let output = workspace.read(cx).run_output(cx);
        !output.is_running() && output.statements().len() == 1
    }));
    assert_eq!(
        phase(&workspace, cx),
        quarry::model::statements::Phase::Finished
    );
}

#[gpui_kit::test]
fn picking_a_tab_closed_while_the_palette_was_open_does_nothing(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    search(cx, "query 2");
    cx.update(|window, cx| {
        let focus = workspace.read(cx).focus_handle(cx);
        focus.dispatch_action(&CloseTab, window, cx);
    });
    cx.update(|_, cx| assert_eq!(workspace.read(cx).tabs().tabs().len(), 1));
    cx.update(|window, cx| {
        let palette = workspace.read(cx).palette().expect("the palette is open");
        let state = palette.read(cx).state().clone();
        state.update(cx, |state, cx| state.focus(window, cx));
    });
    cx.simulate_keystrokes("enter");
    assert!(settle(cx, |window, cx| !window.has_active_dialog(cx)));
    cx.update(|_, cx| assert_eq!(workspace.read(cx).tabs().tabs().len(), 1));
}
```

Modify `crates/app/tests/app/support.rs`:
```diff
@@ -2,7 +2,10 @@
 
 use std::{sync::Arc, time::Duration};
 
-use gpui_kit::{App, Entity, TestAppContext, VisualTestContext, Window, test::TestWindowExt};
+use gpui_kit::{
+    App, AppContext, Entity, TestAppContext, VisualTestContext, Window, base::Root,
+    test::TestWindowExt,
+};
 use quarry::{
     model::statements::Phase,
     sample,
@@ -35,7 +38,8 @@ pub fn open(
 }
 
 /// A workspace on a fresh sample database saving to `store`, with its
-/// editor focused. Call [`install`] first.
+/// editor focused. Like the real window, it sits in gpui-kit's `Root`,
+/// which draws dialogs. Call [`install`] first.
 pub fn open_with(
     cx: &mut TestAppContext,
     store: Arc<dyn DraftStore>,
@@ -45,8 +49,17 @@ pub fn open_with(
         store,
         problem: None,
     };
-    let (workspace, cx) =
-        cx.add_window_view(|window, cx| Workspace::new(session, sample::NAME, saved, window, cx));
+    let mut opened = None;
+    let handle = cx.add_window(|window, cx| {
+        let workspace = cx.new(|cx| Workspace::new(session, sample::NAME, saved, window, cx));
+        opened = Some(workspace.clone());
+        Root::new(workspace, window, cx)
+    });
+    let cx = VisualTestContext::from_window(handle.into(), cx).into_mut();
+    let workspace = opened.ok_or_else(|| EngineError::Open {
+        target: "the test window".to_owned(),
+        message: "it built no workspace".to_owned(),
+    })?;
     cx.update(|window, cx| {
         let editor = workspace.read(cx).editor(cx);
         editor.update(cx, |state, cx| state.focus(window, cx));
```

Modify `crates/app/tests/shell.rs`:
```diff
@@ -1,5 +1,5 @@
-//! Start-up: the main window opens on the sample database, and every command
-//! has a key binding and a menu item.
+//! Start-up: the main window opens on the sample database, the menus come
+//! from the command table, and commands have key bindings.
 
 use std::{cell::Cell, path::Path, rc::Rc};
 
@@ -35,28 +35,42 @@ fn the_window_has_a_transparent_titlebar_and_the_default_size(cx: &mut TestAppCo
 }
 
 #[test]
-fn every_menu_command_has_a_key_binding() {
+fn every_command_but_the_other_copy_formats_has_a_key_binding() {
     let bound: Vec<&str> = actions::key_bindings()
         .iter()
         .map(|binding| binding.action().name())
         .collect();
+    let unbound = ["quarry::CopyCsv", "quarry::CopyJson"];
+    let commands = actions::commands();
+    assert_eq!(commands.len(), 17);
+    for command in commands {
+        let name = (command.action)().name();
+        assert!(
+            bound.contains(&name) != unbound.contains(&name),
+            "{name} has no key binding"
+        );
+    }
+}
+
+#[test]
+fn the_menus_are_the_command_table_in_order() {
     let menus = actions::menus();
-    let commands: Vec<&str> = menus
+    let names: Vec<&str> = menus.iter().map(|menu| menu.name.as_ref()).collect();
+    assert_eq!(names, ["Quarry", "File", "Edit", "Query", "View"]);
+    let items: Vec<String> = menus
         .iter()
         .flat_map(|menu| &menu.items)
         .filter_map(|item| match item {
-            MenuItem::Action { action, .. } => Some(action.name()),
+            MenuItem::Action { name, .. } => Some(name.to_string()),
             _ => None,
         })
         .collect();
-    assert_eq!(commands.len(), 16);
-    let unbound = ["quarry::CopyCsv", "quarry::CopyJson"];
-    for command in commands {
-        assert!(
-            bound.contains(&command) != unbound.contains(&command),
-            "{command} has no key binding"
-        );
-    }
+    let labels: Vec<&str> = actions::commands()
+        .iter()
+        .map(|command| command.label)
+        .collect();
+    assert_eq!(items, labels);
+    assert_eq!(items[3..6], ["Copy as TSV", "Copy as CSV", "Copy as JSON"]);
 }
 
 #[test]
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry palette shell`
Expected: FAIL to compile: no `quarry::views::palette`, no `actions::commands`.

- [ ] **Step 3: Implement**

Modify `crates/app/src/shell/actions.rs`:
```diff
@@ -1,9 +1,10 @@
 //! Every command, its key binding and its menu.
 //!
 //! Adding a command means declaring it in `actions!`, adding one row to
-//! [`key_bindings`] and one menu item to [`menus`].
+//! [`commands`], and a row to [`key_bindings`] if it has a key. The command
+//! table makes the menus and the palette's Commands.
 
-use gpui_kit::{App, KeyBinding, Menu, MenuItem, actions};
+use gpui_kit::{Action, App, KeyBinding, Menu, MenuItem, actions};
 
 actions!(
     quarry,
@@ -38,6 +39,8 @@ actions!(
         CopyCsv,
         /// Copies the grid's selection, or the whole result, as JSON.
         CopyJson,
+        /// Opens the ⌘K palette, or closes it when it is open.
+        OpenPalette,
         /// Quits Quarry.
         Quit,
     ]
@@ -50,6 +53,9 @@ pub const WORKSPACE: &str = "Workspace";
 /// over the editor's built-in ⌘↵ (newline) and ⌘. (code actions).
 pub const WORKSPACE_EDITOR: &str = "Workspace > Input";
 
+/// The palette's key context, where ⌘K closes it again.
+pub const PALETTE: &str = "Palette";
+
 /// The results grid's key context. ⌘C there copies as TSV; the editor
 /// keeps its own ⌘C.
 pub const GRID: &str = "DataTable";
@@ -72,40 +78,82 @@ pub fn key_bindings() -> Vec<KeyBinding> {
         KeyBinding::new("cmd-2", FocusEditor, None),
         KeyBinding::new("cmd-shift-t", CycleTheme, None),
         KeyBinding::new("cmd-c", CopyTsv, Some(GRID)),
+        KeyBinding::new("cmd-k", OpenPalette, Some(WORKSPACE)),
+        KeyBinding::new("cmd-k", OpenPalette, Some(WORKSPACE_EDITOR)),
+        KeyBinding::new("cmd-k", OpenPalette, Some(PALETTE)),
         KeyBinding::new("cmd-q", Quit, None),
     ]
 }
 
-/// The macOS menu bar.
-pub fn menus() -> Vec<Menu> {
+/// The app menu, which holds Quit; the palette leaves it out.
+pub const APP_MENU: &str = "Quarry";
+
+/// One command: its menu item and palette label, its action, and its menu.
+pub struct Command {
+    /// The menu item's text, and the palette's.
+    pub label: &'static str,
+    /// Makes the action the command dispatches.
+    pub action: fn() -> Box<dyn Action>,
+    /// The menu the command is in.
+    pub menu: &'static str,
+}
+
+/// Every command, menu by menu, in menu order.
+pub fn commands() -> Vec<Command> {
+    let command = |label, action, menu| Command {
+        label,
+        action,
+        menu,
+    };
     vec![
-        Menu::new("Quarry").items([MenuItem::action("Quit Quarry", Quit)]),
-        Menu::new("File").items([
-            MenuItem::action("New Tab", NewTab),
-            MenuItem::action("Close Tab", CloseTab),
-        ]),
-        Menu::new("Edit").items([
-            MenuItem::action("Copy as TSV", CopyTsv),
-            MenuItem::action("Copy as CSV", CopyCsv),
-            MenuItem::action("Copy as JSON", CopyJson),
-        ]),
-        Menu::new("Query").items([
-            MenuItem::action("Run Statement", RunStatement),
-            MenuItem::action("Run All", RunAll),
-            MenuItem::action("Explain", Explain),
-            MenuItem::action("Cancel", CancelRun),
-        ]),
-        Menu::new("View").items([
-            MenuItem::action("Next Tab", NextTab),
-            MenuItem::action("Previous Tab", PreviousTab),
-            MenuItem::action("Toggle Sidebar", ToggleSidebar),
-            MenuItem::action("Focus Sidebar", FocusSidebar),
-            MenuItem::action("Focus Editor", FocusEditor),
-            MenuItem::action("Cycle Theme", CycleTheme),
-        ]),
+        command("Quit Quarry", || Box::new(Quit), APP_MENU),
+        command("New Tab", || Box::new(NewTab), "File"),
+        command("Close Tab", || Box::new(CloseTab), "File"),
+        command("Copy as TSV", || Box::new(CopyTsv), "Edit"),
+        command("Copy as CSV", || Box::new(CopyCsv), "Edit"),
+        command("Copy as JSON", || Box::new(CopyJson), "Edit"),
+        command("Run Statement", || Box::new(RunStatement), "Query"),
+        command("Run All", || Box::new(RunAll), "Query"),
+        command("Explain", || Box::new(Explain), "Query"),
+        command("Cancel", || Box::new(CancelRun), "Query"),
+        command("Open Palette", || Box::new(OpenPalette), "View"),
+        command("Next Tab", || Box::new(NextTab), "View"),
+        command("Previous Tab", || Box::new(PreviousTab), "View"),
+        command("Toggle Sidebar", || Box::new(ToggleSidebar), "View"),
+        command("Focus Sidebar", || Box::new(FocusSidebar), "View"),
+        command("Focus Editor", || Box::new(FocusEditor), "View"),
+        command("Cycle Theme", || Box::new(CycleTheme), "View"),
     ]
 }
 
+/// The macOS menu bar, made from [`commands`].
+pub fn menus() -> Vec<Menu> {
+    let commands = commands();
+    let mut names: Vec<&str> = Vec::new();
+    for command in &commands {
+        if !names.contains(&command.menu) {
+            names.push(command.menu);
+        }
+    }
+    names
+        .into_iter()
+        .map(|name| {
+            let items = commands.iter().filter(|command| command.menu == name);
+            Menu::new(name).items(items.map(menu_item))
+        })
+        .collect()
+}
+
+fn menu_item(command: &Command) -> MenuItem {
+    MenuItem::Action {
+        name: command.label.into(),
+        action: (command.action)(),
+        os_action: None,
+        checked: false,
+        disabled: false,
+    }
+}
+
 /// Registers the bindings, the menu bar and the app-wide actions.
 pub fn install(cx: &mut App) {
     cx.bind_keys(key_bindings());
```

Modify `crates/app/src/views/mod.rs`:
```diff
@@ -1,6 +1,7 @@
 //! GPUI views: each renders one region of the workspace.
 
 pub mod editor;
+pub mod palette;
 pub mod poll;
 pub mod query_tab;
 pub mod results;
```

Create `crates/app/src/views/palette/commands.rs`:
```rust
//! Commands, in menu order with their shortcuts; picking one dispatches its
//! action, as its shortcut or menu item does.

use gpui_kit::{Action, App, Context, Window, component::kbd::Kbd};

use super::PaletteSource;
use crate::{
    model::palette::PaletteItem,
    shell::actions::{self, APP_MENU},
    views::workspace::Workspace,
};

/// Every command but those in the app menu.
pub struct Commands;

/// The shortcut that runs `action`, as in "⌘⇧R", or nothing.
fn shortcut(action: &dyn Action, cx: &App) -> String {
    let keymap = cx.key_bindings();
    let keymap = keymap.borrow();
    keymap
        .bindings_for_action(action)
        .last()
        .map(|binding| {
            let keys: Vec<String> = binding
                .keystrokes()
                .iter()
                .map(|key| Kbd::format(key.inner()))
                .collect();
            keys.join(" ")
        })
        .unwrap_or_default()
}

impl PaletteSource for Commands {
    fn id(&self) -> &'static str {
        "commands"
    }

    fn title(&self) -> &'static str {
        "Commands"
    }

    fn items(&self, _: &Workspace, cx: &App) -> Vec<PaletteItem> {
        actions::commands()
            .iter()
            .enumerate()
            .filter(|(_, command)| command.menu != APP_MENU)
            .map(|(index, command)| PaletteItem {
                source: self.id(),
                key: index.to_string(),
                label: command.label.to_owned(),
                detail: shortcut(&*(command.action)(), cx),
            })
            .collect()
    }

    fn pick(&self, key: &str, _: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
        let commands = actions::commands();
        let command = key
            .parse()
            .ok()
            .and_then(|index: usize| commands.get(index));
        if let Some(command) = command {
            window.dispatch_action((command.action)(), cx);
        }
    }
}
```

Create `crates/app/src/views/palette/dialog.rs`:
```rust
//! The palette's dialog: a search field over every source's items, in
//! groups.
//!
//! The items are collected when the palette opens. Each keystroke filters
//! them again with [`palette::filter`]; gpui-component's own filtering is
//! off. Picking an item closes the dialog, then asks its source to act.

use std::rc::Rc;

use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    Render, SharedString, Styled, TestSupportExt, WeakEntity, Window,
    component::{
        IndexPath, WindowExt,
        command::{Command, CommandGroup, CommandItem, CommandState},
    },
    div, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    tokens::{space, text},
};

use super::PaletteSourceRegistry;
use crate::{
    model::palette::{self, PaletteItem},
    shell::actions::{OpenPalette, PALETTE},
    views::workspace::Workspace,
};

/// The palette's width.
pub const PALETTE_WIDTH: f32 = 560.0;

/// What the search field says before anything is typed.
pub const PLACEHOLDER: &str = "Search tabs, tables and commands";

/// One source's matching items, under its heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteGroup {
    /// The source's heading.
    pub title: &'static str,
    /// Its matching items, best first.
    pub items: Vec<PaletteItem>,
}

/// The open palette.
pub struct Palette {
    state: Entity<CommandState>,
    workspace: WeakEntity<Workspace>,
    registry: Rc<PaletteSourceRegistry>,
    items: Vec<PaletteItem>,
    query: String,
}

impl Palette {
    /// The groups the current query shows; a source without matches has
    /// none.
    pub fn groups(&self) -> Vec<PaletteGroup> {
        let mut groups: Vec<PaletteGroup> = Vec::new();
        for item in palette::filter(&self.items, &self.query) {
            match groups.last_mut() {
                Some(group) if group.items[0].source == item.source => {
                    group.items.push(item.clone());
                }
                _ => groups.push(PaletteGroup {
                    title: self.title(item.source),
                    items: vec![item.clone()],
                }),
            }
        }
        groups
    }

    fn title(&self, source: &str) -> &'static str {
        self.registry
            .get(source)
            .map_or("", |source| source.title())
    }

    /// The search field's state, to focus it or type into it.
    pub fn state(&self) -> &Entity<CommandState> {
        &self.state
    }

    fn set_query(&mut self, query: &str, cx: &mut Context<Self>) {
        query.clone_into(&mut self.query);
        cx.notify();
    }

    /// The item at `index` among the groups shown now.
    fn item_at(&self, index: IndexPath) -> Option<PaletteItem> {
        let groups = self.groups();
        groups.get(index.section)?.items.get(index.row).cloned()
    }
}

/// Opens the palette over `workspace`'s window, listing what `registry`'s
/// sources have now, and focuses its search field.
pub fn open(
    workspace: &Workspace,
    registry: Rc<PaletteSourceRegistry>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> Entity<Palette> {
    let items = registry.items(workspace, cx);
    let owner = cx.weak_entity();
    let palette = cx.new(|cx| Palette {
        state: cx.new(|cx| CommandState::new(window, cx)),
        workspace: owner,
        registry,
        items,
        query: String::new(),
    });
    let shown = palette.clone();
    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .w(px(PALETTE_WIDTH))
            .centered()
            .close_button(false)
            .child(shown.clone())
    });
    let state = palette.read(cx).state.clone();
    state.update(cx, |state, cx| state.focus(window, cx));
    palette
}

/// Closes the palette, then lets the picked item's source act on it.
fn confirm(palette: &WeakEntity<Palette>, index: IndexPath, window: &mut Window, cx: &mut App) {
    let Some(palette) = palette.upgrade() else {
        return;
    };
    let picked = palette.read(cx).item_at(index);
    let workspace = palette.read(cx).workspace.clone();
    let registry = Rc::clone(&palette.read(cx).registry);
    window.close_dialog(cx);
    let Some(item) = picked else {
        return;
    };
    let _ = workspace.update(cx, |workspace, cx| {
        if let Some(source) = registry.get(item.source) {
            source.pick(&item.key, workspace, window, cx);
        }
    });
}

impl Render for Palette {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let on_query = cx.weak_entity();
        let on_confirm = cx.weak_entity();
        let command = Command::new(&self.state)
            .filterable(false)
            .bordered(false)
            .placeholder(PLACEHOLDER)
            .empty(|_, _, cx| no_matches(cx))
            .on_query(move |query, _, cx| {
                let _ = on_query.update(cx, |palette, cx| palette.set_query(query, cx));
            })
            .on_confirm(move |index, window, cx| confirm(&on_confirm, index, window, cx));
        let command = self.groups().into_iter().fold(command, |command, group| {
            command.group(command_group(group))
        });
        div()
            .id("palette")
            .key_context(PALETTE)
            .on_action(|_: &OpenPalette, window, cx| window.close_dialog(cx))
            .child(command)
            .test_support()
    }
}

fn command_group(group: PaletteGroup) -> CommandGroup {
    let items = group.items.into_iter().map(|item| {
        let label = SharedString::from(item.label);
        let detail = SharedString::from(item.detail);
        let shown = label.clone();
        CommandItem::new()
            .label(label)
            .child(move |_, cx| row(shown.clone(), detail.clone(), cx))
    });
    CommandGroup::new().label(group.title).items(items)
}

/// An item: its label, and its detail quietly on the right.
fn row(label: SharedString, detail: SharedString, cx: &App) -> AnyElement {
    let colors = Aurora::colors(cx);
    div()
        .w_full()
        .flex()
        .justify_between()
        .gap(px(space::MD))
        .child(label)
        .child(
            div()
                .text_size(px(text::CAPTION))
                .text_color(colors.get(Role::TextMuted))
                .child(detail),
        )
        .into_any_element()
}

/// What the palette says when nothing matches.
fn no_matches(cx: &App) -> AnyElement {
    div()
        .id("palette-empty")
        .py(px(space::LG))
        .flex()
        .justify_center()
        .text_size(px(text::BODY))
        .text_color(Aurora::colors(cx).get(Role::TextMuted))
        .child("No matches")
        .test_support()
        .into_any_element()
}
```

Create `crates/app/src/views/palette/mod.rs`:
```rust
//! The ⌘K palette: open tabs, recently closed tabs, tables and commands.
//!
//! Each kind of item comes from a [`PaletteSource`] in the
//! [`PaletteSourceRegistry`]; adding a source is a new module and one line
//! in [`PaletteSourceRegistry::standard`]. [`dialog`] draws the palette.

mod commands;
pub mod dialog;
mod tabs;

use gpui_kit::{App, Context, Window};

use super::workspace::Workspace;
use crate::model::palette::PaletteItem;

/// One kind of thing the palette lists.
pub trait PaletteSource {
    /// A stable name, such as "tabs".
    fn id(&self) -> &'static str;

    /// The heading over its items.
    fn title(&self) -> &'static str;

    /// Its items now, in the order they are listed.
    fn items(&self, workspace: &Workspace, cx: &App) -> Vec<PaletteItem>;

    /// Acts on the item `key` was listed with. A key that no longer names
    /// anything does nothing.
    fn pick(
        &self,
        key: &str,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    );
}

/// The palette's sources, in the order their groups appear.
pub struct PaletteSourceRegistry {
    sources: Vec<Box<dyn PaletteSource>>,
}

impl PaletteSourceRegistry {
    /// Open tabs, then commands.
    pub fn standard() -> Self {
        Self {
            sources: vec![Box::new(tabs::OpenTabs), Box::new(commands::Commands)],
        }
    }

    /// Every source, in order.
    pub fn sources(&self) -> &[Box<dyn PaletteSource>] {
        &self.sources
    }

    /// The source named `id`.
    pub fn get(&self, id: &str) -> Option<&dyn PaletteSource> {
        self.sources
            .iter()
            .find(|source| source.id() == id)
            .map(AsRef::as_ref)
    }

    /// Every source's items, source by source.
    pub fn items(&self, workspace: &Workspace, cx: &App) -> Vec<PaletteItem> {
        self.sources
            .iter()
            .flat_map(|source| source.items(workspace, cx))
            .collect()
    }
}
```

Create `crates/app/src/views/palette/tabs.rs`:
```rust
//! Open tabs, in tab order; picking one activates it.

use gpui_kit::{App, Context, Window};

use super::PaletteSource;
use crate::{model::palette::PaletteItem, views::workspace::Workspace};

/// The open query tabs.
pub struct OpenTabs;

impl PaletteSource for OpenTabs {
    fn id(&self) -> &'static str {
        "tabs"
    }

    fn title(&self) -> &'static str {
        "Open tabs"
    }

    fn items(&self, workspace: &Workspace, _: &App) -> Vec<PaletteItem> {
        let active = workspace.tabs().active_id();
        workspace
            .tabs()
            .tabs()
            .iter()
            .map(|tab| PaletteItem {
                source: self.id(),
                key: tab.id.to_string(),
                label: tab.title.clone(),
                detail: if tab.id == active { "current" } else { "" }.to_owned(),
            })
            .collect()
    }

    fn pick(
        &self,
        key: &str,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        if let Ok(id) = key.parse() {
            workspace.activate_tab(id, window, cx);
        }
    }
}
```

Modify `crates/app/src/views/workspace/layout.rs`:
```diff
@@ -47,6 +47,7 @@ impl Render for Workspace {
             .on_action(cx.listener(Self::copy_tsv))
             .on_action(cx.listener(Self::copy_csv))
             .on_action(cx.listener(Self::copy_json))
+            .on_action(cx.listener(Self::open_palette))
             .size_full()
             .flex()
             .bg(window_background(&colors))
```

Modify `crates/app/src/views/workspace/mod.rs`:
```diff
@@ -7,19 +7,21 @@
 mod copy;
 mod drafts;
 mod layout;
+mod palette;
 mod runs;
 mod tab_strip;
 mod tabs;
 
 use std::{
     collections::BTreeMap,
+    rc::Rc,
     sync::{Arc, Mutex},
     time::Duration,
 };
 
 use gpui_kit::{
-    App, AppContext, Context, Entity, FocusHandle, Focusable, Subscription, Task, Window,
-    component::input::EditorState,
+    App, AppContext, Context, Entity, FocusHandle, Focusable, Subscription, Task, WeakEntity,
+    Window, component::input::EditorState,
 };
 use quarry_engine::session::Session;
 use quarry_store::drafts::{Draft, DraftStore, Drafts};
@@ -30,7 +32,12 @@ pub use self::{
     drafts::{AUTOSAVE, SavedTabs, save_in_order},
     tab_strip::tab_element_id,
 };
-use super::{editor, query_tab::QueryTab, schema::SchemaTree};
+use super::{
+    editor,
+    palette::{PaletteSourceRegistry, dialog::Palette},
+    query_tab::QueryTab,
+    schema::SchemaTree,
+};
 use crate::{
     model::{
         save_order::SaveOrder,
@@ -67,6 +74,8 @@ pub struct Workspace {
     schema_read: Option<Task<()>>,
     note: Option<String>,
     note_timer: Option<Task<()>>,
+    palette: Rc<PaletteSourceRegistry>,
+    shown_palette: WeakEntity<Palette>,
     _quit: Subscription,
 }
 
@@ -99,6 +108,8 @@ impl Workspace {
             schema_read: None,
             note: None,
             note_timer: None,
+            palette: Rc::new(PaletteSourceRegistry::standard()),
+            shown_palette: WeakEntity::new_invalid(),
             _quit: cx.on_app_quit(|workspace, cx| {
                 workspace.save_before_quit(cx);
                 async {}
```

Create `crates/app/src/views/workspace/palette.rs`:
```rust
//! Opening the ⌘K palette over the workspace.

use std::rc::Rc;

use gpui_kit::{Context, Entity, Window};

use super::Workspace;
use crate::{
    shell::actions::OpenPalette,
    views::palette::dialog::{self, Palette},
};

impl Workspace {
    pub(super) fn open_palette(
        &mut self,
        _: &OpenPalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let registry = Rc::clone(&self.palette);
        let palette = dialog::open(self, registry, window, cx);
        self.shown_palette = palette.downgrade();
    }

    /// The palette, while it is open.
    pub fn palette(&self) -> Option<Entity<Palette>> {
        self.shown_palette.upgrade()
    }
}
```

- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry`
Expected: every test passes, including the seven palette tests, `every_command_but_the_other_copy_formats_has_a_key_binding` and `the_menus_are_the_command_table_in_order`. The Root-wrapped support window leaves every earlier test passing.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/app/src/shell/actions.rs crates/app/src/views/mod.rs crates/app/src/views/palette/commands.rs crates/app/src/views/palette/dialog.rs crates/app/src/views/palette/mod.rs crates/app/src/views/palette/tabs.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/mod.rs crates/app/src/views/workspace/palette.rs crates/app/tests/app/main.rs crates/app/tests/app/palette.rs crates/app/tests/app/support.rs crates/app/tests/shell.rs
git commit -m "feat(app): add the command palette with open tabs and commands"
```

---

### Task 8: Palette sources: Tables & views, and Recently closed

The last two sources join the registry, which is now Open tabs, Recently closed, Tables & views, Commands.

**Recently closed:**
- ⌘W remembers the closed tab through `closed_tabs::remember`, including the only tab before a fresh "Query N" replaces it.
- Each item shows the tab's title, with its first non-blank line as detail.
- Picking one reopens it after the active tab, with its title and text, removes it from the list and saves at once.
- It works from the in-memory list while drafts aren't being saved.

**Tables & views:**
- Each item shows "table N columns" or "view N columns".
- Picking one opens the sidebar, expands the table, scrolls it into view and focuses its row.
- To make that possible, `SchemaTree` owns its scroll and gives each table row a focus handle, through the new `SidebarItem::track_focus`.
- A table that is gone re-reads the schema and focuses nothing.

**Files:**
- Modify: `crates/app/src/model/closed_tabs.rs`
- Modify: `crates/app/src/model/schema_tree.rs`
- Modify: `crates/app/src/model/tabs.rs`
- Create: `crates/app/src/views/palette/closed.rs`
- Modify: `crates/app/src/views/palette/mod.rs`
- Create: `crates/app/src/views/palette/tables.rs`
- Modify: `crates/app/src/views/schema.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Modify: `crates/app/src/views/workspace/tabs.rs`
- Modify: `crates/app/tests/app/palette.rs`
- Modify: `crates/app/tests/app/tabs.rs`
- Modify: `crates/ui-kit/src/components/sidebar_item.rs`
- Modify: `crates/ui-kit/tests/keyboard.rs`

**Interfaces:**
- Consumes: `model::closed_tabs` (Task 3), `PaletteSource` (Task 7), `Workspace::closed` (Task 2).
- Produces:
  - **Models:**
    - `TabList::reopen(&mut self, title: String) -> TabId`.
    - `closed_tabs::first_line(&str) -> &str`.
    - `schema_tree::{row_of(&Schema, &BTreeSet<String>, &str) -> Option<usize>, summary(&Table) -> String}`.
  - **ui-kit:** `SidebarItem::track_focus(FocusHandle)`.
  - **Schema tree:** `SchemaTree::{tables(&self) -> &[Table], reveal(&mut self, &str, &mut Window, cx) -> bool, focused_table(&self, &Window) -> Option<&str>}`. The tree's element id is `schema`.
  - **Workspace:** `Workspace::{closed_tabs(&self) -> &[Draft], reopen_tab(&mut self, usize, window, cx), reveal_table(&mut self, &str, window, cx)}`.

- [ ] **Step 1: Write the failing tests**

Modify `crates/app/tests/app/palette.rs`:
```diff
@@ -72,16 +72,17 @@ fn command_k_again_closes_the_palette(cx: &mut TestAppContext) {
 }
 
 #[gpui_kit::test]
-fn an_empty_query_lists_open_tabs_then_commands(cx: &mut TestAppContext) {
+fn an_empty_query_lists_tabs_then_tables_then_commands(cx: &mut TestAppContext) {
     let (workspace, cx) = open(cx).expect("the sample database opens");
     cx.simulate_keystrokes("cmd-t");
     search(cx, "");
     let groups = groups(&workspace, cx);
     let headings: Vec<&str> = groups.iter().map(|(title, _)| title.as_str()).collect();
-    assert_eq!(headings, ["Open tabs", "Commands"]);
+    assert_eq!(headings, ["Open tabs", "Tables & views", "Commands"]);
     assert_eq!(groups[0].1, ["Query 1", "Query 2"]);
-    assert_eq!(groups[1].1[..3], ["New Tab", "Close Tab", "Copy as TSV"]);
-    assert!(!groups[1].1.contains(&"Quit Quarry".to_owned()));
+    assert!(groups[1].1.contains(&"astronauts".to_owned()));
+    assert_eq!(groups[2].1[..3], ["New Tab", "Close Tab", "Copy as TSV"]);
+    assert!(!groups[2].1.contains(&"Quit Quarry".to_owned()));
     let current = cx.update(|_, cx| {
         let palette = workspace.read(cx).palette();
         palette.map(|p| p.read(cx).groups()[0].items[1].detail.clone())
@@ -148,3 +149,48 @@ fn picking_a_tab_closed_while_the_palette_was_open_does_nothing(cx: &mut TestApp
     assert!(settle(cx, |window, cx| !window.has_active_dialog(cx)));
     cx.update(|_, cx| assert_eq!(workspace.read(cx).tabs().tabs().len(), 1));
 }
+
+#[gpui_kit::test]
+fn tables_and_views_show_their_kind_and_columns(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    search(cx, "mission_crews");
+    let detail = cx.update(|_, cx| {
+        let palette = workspace.read(cx).palette();
+        palette.map(|p| p.read(cx).groups()[0].items[0].detail.clone())
+    });
+    assert!(detail.is_some_and(|d| d.starts_with("view ") && d.ends_with(" columns")));
+}
+
+#[gpui_kit::test]
+fn picking_a_table_opens_the_sidebar_and_focuses_its_row(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    cx.simulate_keystrokes("cmd-\\");
+    cx.update(|_, cx| assert!(!workspace.read(cx).sidebar_open()));
+    search(cx, "telemetry");
+    cx.simulate_keystrokes("enter");
+    assert!(settle(cx, |window, cx| {
+        let schema = workspace.read(cx).schema().read(cx);
+        schema.focused_table(window) == Some("telemetry")
+    }));
+    cx.update(|_, cx| {
+        let workspace = workspace.read(cx);
+        assert!(workspace.sidebar_open());
+        assert!(workspace.schema().read(cx).expanded().contains("telemetry"));
+    });
+}
+
+#[gpui_kit::test]
+fn a_table_that_is_gone_focuses_nothing(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    cx.update(|window, cx| {
+        workspace.update(cx, |workspace, cx| {
+            workspace.reveal_table("nowhere", window, cx);
+        });
+    });
+    assert!(settle(cx, |_, _| true));
+    cx.update(|window, cx| {
+        let schema = workspace.read(cx).schema().read(cx);
+        assert_eq!(schema.focused_table(window), None);
+        assert!(schema.expanded().is_empty());
+    });
+}
```

Modify `crates/app/tests/app/tabs.rs`:
```diff
@@ -6,7 +6,9 @@ use std::sync::{
     atomic::{AtomicBool, AtomicUsize, Ordering},
 };
 
-use gpui_kit::{Entity, TestAppContext, VisualTestContext, test::TestWindowExt};
+use gpui_kit::{
+    Entity, TestAppContext, VisualTestContext, component::WindowExt, test::TestWindowExt,
+};
 use quarry::{
     model::{save_order::SaveOrder, statements::Phase},
     views::workspace::{AUTOSAVE, Workspace, save_in_order, tab_element_id},
@@ -17,7 +19,7 @@ use quarry_store::{
     sqlite::SqliteStore,
 };
 
-use super::support::{drawn, install, open_with as open, settle};
+use super::support::{drawn, install, open_with as open, settle, write};
 
 /// A store whose every read and write fails.
 struct Broken;
@@ -341,3 +343,80 @@ fn a_delayed_older_snapshot_is_skipped() {
     );
     assert_eq!(store.load(), Ok(snapshot("newer")));
 }
+
+fn closed_titles(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Vec<String> {
+    cx.update(|_, cx| {
+        let closed = workspace.read(cx).closed_tabs();
+        closed.iter().map(|draft| draft.title.clone()).collect()
+    })
+}
+
+/// Opens the palette, types `query` and picks the first match.
+fn pick(cx: &mut VisualTestContext, query: &str) {
+    cx.simulate_keystrokes("cmd-k");
+    cx.simulate_input(query);
+    cx.simulate_keystrokes("enter");
+    assert!(settle(cx, |window, cx| !window.has_active_dialog(cx)));
+}
+
+#[gpui_kit::test]
+fn a_closed_tab_reopens_from_the_palette_and_saves_at_once(cx: &mut TestAppContext) {
+    install(cx);
+    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
+    let (workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
+    cx.simulate_keystrokes("cmd-t cmd-t cmd-{");
+    write(&workspace, cx, "\n  SELECT 42\nFROM astronauts");
+    cx.simulate_keystrokes("cmd-w cmd-{");
+    assert_eq!(titles(&workspace, cx), ["Query 1", "Query 3"]);
+    assert_eq!(closed_titles(&workspace, cx), ["Query 2"]);
+    pick(cx, "query 2");
+    assert_eq!(titles(&workspace, cx), ["Query 1", "Query 2", "Query 3"]);
+    assert_eq!(active(&workspace, cx), 1);
+    assert_eq!(text(&workspace, cx), "\n  SELECT 42\nFROM astronauts");
+    assert!(closed_titles(&workspace, cx).is_empty());
+    let saved = store.load().expect("the drafts load");
+    assert_eq!((saved.tabs.len(), saved.closed.len()), (3, 0));
+}
+
+#[gpui_kit::test]
+fn the_palette_shows_a_closed_tab_with_its_first_line(cx: &mut TestAppContext) {
+    install(cx);
+    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
+    let (workspace, cx) = open(cx, store).expect("the sample database opens");
+    cx.simulate_keystrokes("cmd-t");
+    write(&workspace, cx, "\n  SELECT 42\nFROM astronauts");
+    cx.simulate_keystrokes("cmd-w cmd-k");
+    cx.run_until_parked();
+    let group = cx.update(|_, cx| {
+        let palette = workspace.read(cx).palette().expect("the palette is open");
+        palette.read(cx).groups()[1].clone()
+    });
+    assert_eq!(group.title, "Recently closed");
+    assert_eq!(group.items[0].label, "Query 2");
+    assert_eq!(group.items[0].detail, "SELECT 42");
+}
+
+#[gpui_kit::test]
+fn closing_the_only_tab_remembers_it(cx: &mut TestAppContext) {
+    install(cx);
+    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
+    let (workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
+    cx.simulate_keystrokes("cmd-w");
+    assert_eq!(titles(&workspace, cx), ["Query 1"]);
+    let closed = cx.update(|_, cx| workspace.read(cx).closed_tabs().to_vec());
+    assert_eq!(closed.len(), 1);
+    assert!(closed[0].text.starts_with("-- Welcome"));
+    assert!(settle(cx, |_, _| true));
+    assert_eq!(store.load().expect("the drafts load").closed.len(), 1);
+}
+
+#[gpui_kit::test]
+fn reopening_works_while_drafts_are_not_saved(cx: &mut TestAppContext) {
+    let (workspace, cx) = super::support::open(cx).expect("the sample database opens");
+    cx.simulate_keystrokes("cmd-t");
+    write(&workspace, cx, "SELECT 7");
+    cx.simulate_keystrokes("cmd-w");
+    pick(cx, "query 2");
+    assert_eq!(titles(&workspace, cx), ["Query 1", "Query 2"]);
+    assert_eq!(text(&workspace, cx), "SELECT 7");
+}
```

Modify `crates/ui-kit/tests/keyboard.rs`:
```diff
@@ -83,3 +83,41 @@ fn space_clicks_and_the_next_stop_is_the_tab(cx: &mut TestAppContext) {
     press(cx, "space");
     assert_eq!((presses.row.get(), presses.tab.get()), (0, 1));
 }
+
+struct Tracked {
+    handle: gpui_kit::FocusHandle,
+    presses: Rc<Presses>,
+}
+
+impl Render for Tracked {
+    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
+        let row = Rc::clone(&self.presses);
+        div().size_full().child(
+            SidebarItem::new("tracked", "Tracked")
+                .track_focus(self.handle.clone())
+                .on_click(move |_, _, _| row.row.set(row.row.get() + 1)),
+        )
+    }
+}
+
+#[gpui_kit::test]
+fn a_row_with_a_tracked_handle_takes_focus_from_code(cx: &mut TestAppContext) {
+    let installed = cx.update(|cx| {
+        gpui_kit::init(cx);
+        Aurora::install(Preference::Dark, cx)
+    });
+    assert_eq!(installed, Ok(()));
+    let presses = Rc::new(Presses::default());
+    let view = Rc::clone(&presses);
+    let (tracked, cx) = cx.add_window_view(move |_, cx| Tracked {
+        handle: cx.focus_handle().tab_index(0).tab_stop(true),
+        presses: view,
+    });
+    cx.update(|window, cx| {
+        let handle = tracked.read(cx).handle.clone();
+        window.focus(&handle, cx);
+        window.draw(cx).clear(cx);
+    });
+    press(cx, "enter");
+    assert_eq!(presses.row.get(), 1);
+}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry-ui-kit -p quarry`
Expected: FAIL to compile: no `track_focus` on `SidebarItem`, no `closed_tabs`, `reveal_table` or `focused_table`.

- [ ] **Step 3: Implement**

Modify `crates/app/src/model/closed_tabs.rs`:
```diff
@@ -18,6 +18,15 @@ pub fn remember(closed: &mut Vec<Draft>, draft: Draft) {
     closed.truncate(CLOSED_LIMIT);
 }
 
+/// The first line of `text` that isn't blank, trimmed: what the palette
+/// shows beside a closed tab's title.
+pub fn first_line(text: &str) -> &str {
+    text.lines()
+        .map(str::trim)
+        .find(|line| !line.is_empty())
+        .unwrap_or_default()
+}
+
 /// Removes and returns entry `index`, if there is one.
 pub fn take(closed: &mut Vec<Draft>, index: usize) -> Option<Draft> {
     (index < closed.len()).then(|| closed.remove(index))
@@ -27,7 +36,7 @@ pub fn take(closed: &mut Vec<Draft>, index: usize) -> Option<Draft> {
 mod tests {
     use quarry_store::drafts::Draft;
 
-    use super::{CLOSED_LIMIT, remember, take};
+    use super::{CLOSED_LIMIT, first_line, remember, take};
 
     fn draft(title: &str, text: &str) -> Draft {
         Draft {
@@ -83,6 +92,13 @@ mod tests {
         assert_eq!(closed[CLOSED_LIMIT - 1].title, "Query 1");
     }
 
+    #[test]
+    fn the_first_line_skips_blank_lines_and_trims() {
+        assert_eq!(first_line("\n  \n  SELECT name\nFROM crew"), "SELECT name");
+        assert_eq!(first_line("one"), "one");
+        assert_eq!(first_line(" \n "), "");
+    }
+
     #[test]
     fn take_removes_one_entry() {
         let mut closed = vec![draft("Query 1", "a"), draft("Query 2", "b")];
```

Modify `crates/app/src/model/schema_tree.rs`:
```diff
@@ -38,6 +38,26 @@ pub fn rows(schema: &Schema, expanded: &BTreeSet<String>) -> Vec<TreeRow> {
     rows
 }
 
+/// The position of table `name`'s row among [`rows`], if it is listed.
+pub fn row_of(schema: &Schema, expanded: &BTreeSet<String>, name: &str) -> Option<usize> {
+    rows(schema, expanded)
+        .iter()
+        .position(|row| matches!(row, TreeRow::Table { name: listed, .. } if listed == name))
+}
+
+/// What the palette says about a table: "table 3 columns" or "view 1
+/// column".
+pub fn summary(table: &Table) -> String {
+    let kind = match table.kind {
+        TableKind::Table => "table",
+        TableKind::View => "view",
+    };
+    match table.columns.len() {
+        1 => format!("{kind} 1 column"),
+        n => format!("{kind} {n} columns"),
+    }
+}
+
 /// Expands `name` when collapsed and collapses it when expanded.
 pub fn toggle(expanded: &mut BTreeSet<String>, name: &str) {
     if !expanded.remove(name) {
@@ -83,7 +103,7 @@ mod tests {
 
     use quarry_engine::schema::{ColumnInfo, Schema, Table, TableKind};
 
-    use super::{TreeRow, rows, toggle};
+    use super::{TreeRow, row_of, rows, summary, toggle};
 
     fn column(name: &str, type_name: &str, primary_key: bool) -> ColumnInfo {
         ColumnInfo {
@@ -158,6 +178,24 @@ mod tests {
         );
     }
 
+    #[test]
+    fn finds_a_table_row_below_expanded_tables() {
+        let expanded = BTreeSet::from(["missions".to_owned()]);
+        assert_eq!(row_of(&schema(), &expanded, "missions"), Some(0));
+        assert_eq!(row_of(&schema(), &expanded, "notes"), Some(3));
+        assert_eq!(row_of(&schema(), &BTreeSet::new(), "roster"), Some(2));
+        assert_eq!(row_of(&schema(), &expanded, "id"), None);
+        assert_eq!(row_of(&schema(), &expanded, "nowhere"), None);
+    }
+
+    #[test]
+    fn summaries_name_the_kind_and_count_columns() {
+        let tables = schema().tables;
+        assert_eq!(summary(&tables[0]), "table 2 columns");
+        assert_eq!(summary(&tables[1]), "table 1 column");
+        assert_eq!(summary(&tables[2]), "view 1 column");
+    }
+
     #[test]
     fn toggling_twice_collapses_again() {
         let mut expanded = BTreeSet::new();
```

Modify `crates/app/src/model/tabs.rs`:
```diff
@@ -60,9 +60,19 @@ impl TabList {
 
     /// Opens a new "Query N" right after the active tab and activates it.
     pub fn open(&mut self) -> TabId {
+        let title = format!("{TITLE_PREFIX}{}", self.next_number());
+        self.insert(title)
+    }
+
+    /// Opens a closed tab again as a new tab titled `title`, right after the
+    /// active tab, and activates it.
+    pub fn reopen(&mut self, title: String) -> TabId {
+        self.insert(title)
+    }
+
+    fn insert(&mut self, title: String) -> TabId {
         let id = self.next_id;
         self.next_id += 1;
-        let title = format!("{TITLE_PREFIX}{}", self.next_number());
         let at = if self.tabs.is_empty() {
             0
         } else {
@@ -236,6 +246,15 @@ mod tests {
         assert_eq!(list.active(), 1);
     }
 
+    #[test]
+    fn reopen_inserts_after_the_active_tab_with_its_title_and_a_new_id() {
+        let mut list = three();
+        assert_eq!(list.reopen("Notes".to_owned()), 10);
+        assert_eq!(titles(&list), ["Query 1", "Query 2", "Notes", "Notes"]);
+        assert_eq!(list.active(), 2);
+        assert_eq!(list.active_id(), 10);
+    }
+
     #[test]
     fn activate_selects_an_open_tab_only() {
         let mut list = three();
```

Create `crates/app/src/views/palette/closed.rs`:
```rust
//! Recently closed tabs, newest first; picking one reopens it.

use gpui_kit::{App, Context, Window};

use super::PaletteSource;
use crate::{
    model::{closed_tabs, palette::PaletteItem},
    views::workspace::Workspace,
};

/// The tabs closed with ⌘W, kept across restarts.
pub struct RecentlyClosed;

impl PaletteSource for RecentlyClosed {
    fn id(&self) -> &'static str {
        "closed"
    }

    fn title(&self) -> &'static str {
        "Recently closed"
    }

    fn items(&self, workspace: &Workspace, _: &App) -> Vec<PaletteItem> {
        workspace
            .closed_tabs()
            .iter()
            .enumerate()
            .map(|(index, draft)| PaletteItem {
                source: self.id(),
                key: index.to_string(),
                label: draft.title.clone(),
                detail: closed_tabs::first_line(&draft.text).to_owned(),
            })
            .collect()
    }

    fn pick(
        &self,
        key: &str,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        if let Ok(index) = key.parse() {
            workspace.reopen_tab(index, window, cx);
        }
    }
}
```

Modify `crates/app/src/views/palette/mod.rs`:
```diff
@@ -4,8 +4,10 @@
 //! [`PaletteSourceRegistry`]; adding a source is a new module and one line
 //! in [`PaletteSourceRegistry::standard`]. [`dialog`] draws the palette.
 
+mod closed;
 mod commands;
 pub mod dialog;
+mod tables;
 mod tabs;
 
 use gpui_kit::{App, Context, Window};
@@ -41,10 +43,15 @@ pub struct PaletteSourceRegistry {
 }
 
 impl PaletteSourceRegistry {
-    /// Open tabs, then commands.
+    /// Open tabs, recently closed tabs, tables and views, then commands.
     pub fn standard() -> Self {
         Self {
-            sources: vec![Box::new(tabs::OpenTabs), Box::new(commands::Commands)],
+            sources: vec![
+                Box::new(tabs::OpenTabs),
+                Box::new(closed::RecentlyClosed),
+                Box::new(tables::Tables),
+                Box::new(commands::Commands),
+            ],
         }
     }
 
```

Create `crates/app/src/views/palette/tables.rs`:
```rust
//! Tables and views, in sidebar order; picking one reveals it in the
//! sidebar.

use gpui_kit::{App, Context, Window};

use super::PaletteSource;
use crate::{
    model::{palette::PaletteItem, schema_tree},
    views::workspace::Workspace,
};

/// The tables and views of the database.
pub struct Tables;

impl PaletteSource for Tables {
    fn id(&self) -> &'static str {
        "tables"
    }

    fn title(&self) -> &'static str {
        "Tables & views"
    }

    fn items(&self, workspace: &Workspace, cx: &App) -> Vec<PaletteItem> {
        workspace
            .schema()
            .read(cx)
            .tables()
            .iter()
            .map(|table| PaletteItem {
                source: self.id(),
                key: table.name.clone(),
                label: table.name.clone(),
                detail: schema_tree::summary(table),
            })
            .collect()
    }

    fn pick(
        &self,
        key: &str,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        workspace.reveal_table(key, window, cx);
    }
}
```

Modify `crates/app/src/views/schema.rs`:
```diff
@@ -1,11 +1,18 @@
 //! The sidebar's schema tree: tables and views, expandable to their columns.
+//!
+//! The tree scrolls on its own, and each table's row has a focus handle, so
+//! the palette can reveal a table: expand it, scroll to it and focus it.
 
-use std::collections::BTreeSet;
+use std::collections::{BTreeMap, BTreeSet};
 
 use gpui_kit::{
-    AnyElement, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px,
+    AnyElement, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
+    ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window, div, px,
+};
+use quarry_engine::{
+    error::EngineError,
+    schema::{Schema, Table},
 };
-use quarry_engine::{error::EngineError, schema::Schema};
 use quarry_ui_kit::{
     Aurora, Role,
     components::sidebar_item::SidebarItem,
@@ -18,6 +25,8 @@ use crate::model::schema_tree::{self, TreeRow};
 pub struct SchemaTree {
     schema: Result<Schema, EngineError>,
     expanded: BTreeSet<String>,
+    scroll: ScrollHandle,
+    focus: BTreeMap<String, FocusHandle>,
 }
 
 impl Default for SchemaTree {
@@ -25,6 +34,8 @@ impl Default for SchemaTree {
         Self {
             schema: Ok(Schema::default()),
             expanded: BTreeSet::new(),
+            scroll: ScrollHandle::new(),
+            focus: BTreeMap::new(),
         }
     }
 }
@@ -41,18 +52,61 @@ impl SchemaTree {
         &self.expanded
     }
 
+    /// The tables and views, in sidebar order; none when the schema could
+    /// not be read.
+    pub fn tables(&self) -> &[Table] {
+        self.schema.as_ref().map_or(&[], |schema| &schema.tables)
+    }
+
+    /// Expands table `name`, scrolls its row into view and focuses it.
+    /// Returns false, changing nothing, when there is no such table.
+    pub fn reveal(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
+        let Ok(schema) = &self.schema else {
+            return false;
+        };
+        let mut expanded = self.expanded.clone();
+        expanded.insert(name.to_owned());
+        let Some(row) = schema_tree::row_of(schema, &expanded, name) else {
+            return false;
+        };
+        self.expanded = expanded;
+        self.scroll.scroll_to_item(row);
+        window.focus(&self.focus_handle(name, cx), cx);
+        cx.notify();
+        true
+    }
+
+    /// The table whose row has focus, if one does.
+    pub fn focused_table(&self, window: &Window) -> Option<&str> {
+        self.focus
+            .iter()
+            .find(|(_, handle)| handle.is_focused(window))
+            .map(|(name, _)| name.as_str())
+    }
+
+    /// Table `name`'s focus handle, made the first time it is asked for.
+    fn focus_handle(&mut self, name: &str, cx: &mut Context<Self>) -> FocusHandle {
+        self.focus
+            .entry(name.to_owned())
+            .or_insert_with(|| cx.focus_handle().tab_index(0).tab_stop(true))
+            .clone()
+    }
+
     fn toggle(&mut self, name: &str, cx: &mut Context<Self>) {
         schema_tree::toggle(&mut self.expanded, name);
         cx.notify();
     }
 
-    fn row(row: TreeRow, cx: &mut Context<Self>) -> AnyElement {
+    fn row(&mut self, row: TreeRow, cx: &mut Context<Self>) -> AnyElement {
         match row {
             TreeRow::Table {
                 name,
                 detail,
                 expanded,
-            } => table_row(name, detail, expanded, cx),
+            } => {
+                let focus = self.focus_handle(&name, cx);
+                table_row(name, detail, expanded, focus, cx)
+            }
             TreeRow::Column { name, detail } => column_row(name, detail, cx),
         }
     }
@@ -67,12 +121,14 @@ fn table_row(
     name: String,
     detail: String,
     expanded: bool,
+    focus: FocusHandle,
     cx: &mut Context<SchemaTree>,
 ) -> AnyElement {
     let label = SharedString::from(name.clone());
     SidebarItem::new(table_row_id(&name), label)
         .trailing(detail)
         .selected(expanded)
+        .track_focus(focus)
         .on_click(cx.listener(move |tree, _, _, cx| tree.toggle(&name, cx)))
         .into_any_element()
 }
@@ -95,18 +151,25 @@ fn column_row(name: String, detail: String, cx: &Context<SchemaTree>) -> AnyElem
 impl Render for SchemaTree {
     fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
         let colors = Aurora::colors(cx);
-        let list = div().flex().flex_col().gap(px(1.));
-        match &self.schema {
-            Err(error) => list
-                .text_size(px(text::SMALL))
-                .text_color(colors.get(Role::Danger))
-                .child(format!("Cannot read the schema: {error}")),
-            Ok(schema) => {
-                let rows = schema_tree::rows(schema, &self.expanded);
-                let children: Vec<AnyElement> =
-                    rows.into_iter().map(|row| Self::row(row, cx)).collect();
-                list.children(children)
+        let list = div()
+            .id("schema")
+            .flex_1()
+            .min_h_0()
+            .overflow_y_scroll()
+            .track_scroll(&self.scroll)
+            .flex()
+            .flex_col()
+            .gap(px(1.));
+        let rows = match &self.schema {
+            Err(error) => {
+                return list
+                    .text_size(px(text::SMALL))
+                    .text_color(colors.get(Role::Danger))
+                    .child(format!("Cannot read the schema: {error}"));
             }
-        }
+            Ok(schema) => schema_tree::rows(schema, &self.expanded),
+        };
+        let children: Vec<AnyElement> = rows.into_iter().map(|row| self.row(row, cx)).collect();
+        list.children(children)
     }
 }
```

Modify `crates/app/src/views/workspace/layout.rs`:
```diff
@@ -71,7 +71,6 @@ impl Workspace {
             .flex()
             .flex_col()
             .gap(px(space::SM))
-            .overflow_y_scroll()
             .child(
                 div()
                     .px(px(space::SM))
```

Modify `crates/app/src/views/workspace/mod.rs`:
```diff
@@ -175,6 +175,21 @@ impl Workspace {
         cx.notify();
     }
 
+    /// Opens the sidebar if it is closed, then expands table `name`,
+    /// scrolls to it and focuses its row. A table that is gone re-reads the
+    /// schema and focuses nothing.
+    pub fn reveal_table(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
+        let revealed = self
+            .schema
+            .update(cx, |tree, cx| tree.reveal(name, window, cx));
+        if !revealed {
+            self.read_schema(cx);
+            return;
+        }
+        self.sidebar_open = true;
+        cx.notify();
+    }
+
     fn focus_editor(&mut self, _: &FocusEditor, window: &mut Window, cx: &mut Context<Self>) {
         self.editor(cx)
             .update(cx, |state, cx| state.focus(window, cx));
```

Modify `crates/app/src/views/workspace/tabs.rs`:
```diff
@@ -1,10 +1,11 @@
-//! The workspace's query tabs: opening, closing and switching.
+//! The workspace's query tabs: opening, closing, reopening and switching.
 
 use gpui_kit::{AppContext, Context, Entity, Window};
+use quarry_store::drafts::Draft;
 
 use super::Workspace;
 use crate::{
-    model::tabs::TabId,
+    model::{closed_tabs, tabs::TabId},
     shell::actions::{CloseTab, NewTab, NextTab, PreviousTab},
     views::query_tab::{QueryTab, QueryTabEvent},
 };
@@ -39,8 +40,11 @@ impl Workspace {
         self.tabs_changed(window, cx);
     }
 
+    /// Closes the active tab and remembers it for reopening, even when it
+    /// is the only one and a fresh tab takes its place.
     pub(super) fn close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
         let closing = self.tabs.active_id();
+        self.remember(closing, cx);
         let opened = self.tabs.close(closing);
         let closed = self.views.remove(&closing);
         // A run that is cut off never emits `RunEnded`, so read the schema for it.
@@ -53,6 +57,37 @@ impl Workspace {
         self.tabs_changed(window, cx);
     }
 
+    /// Puts tab `id`'s title and text at the front of the closed tabs.
+    fn remember(&mut self, id: TabId, cx: &mut Context<Self>) {
+        let tab = self.tabs.tabs().iter().find(|tab| tab.id == id);
+        let Some((tab, view)) = tab.zip(self.views.get(&id)) else {
+            return;
+        };
+        let draft = Draft {
+            id,
+            title: tab.title.clone(),
+            text: view.read(cx).text(cx),
+        };
+        closed_tabs::remember(&mut self.closed, draft);
+    }
+
+    /// The recently closed tabs, newest first.
+    pub fn closed_tabs(&self) -> &[Draft] {
+        &self.closed
+    }
+
+    /// Reopens closed tab `index` as a new tab after the active one, with
+    /// its title and text, and saves at once. An index past the list does
+    /// nothing.
+    pub fn reopen_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
+        let Some(draft) = closed_tabs::take(&mut self.closed, index) else {
+            return;
+        };
+        let id = self.tabs.reopen(draft.title);
+        self.add_view(id, &draft.text, window, cx);
+        self.tabs_changed(window, cx);
+    }
+
     pub(super) fn next_tab(&mut self, _: &NextTab, window: &mut Window, cx: &mut Context<Self>) {
         self.tabs.next();
         self.tabs_changed(window, cx);
```

Modify `crates/ui-kit/src/components/sidebar_item.rs`:
```diff
@@ -1,12 +1,15 @@
 //! A sidebar row: label, optional trailing text, selectable.
 //!
 //! A row with a click handler is a keyboard stop (see [`keyboard_focusable`]).
+//! Give it a focus handle with [`SidebarItem::track_focus`] to focus it from
+//! code.
 
 use std::rc::Rc;
 
 use gpui_kit::{
-    App, ClickEvent, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce,
-    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
+    App, ClickEvent, Div, ElementId, FocusHandle, Hsla, InteractiveElement, IntoElement,
+    ParentElement, RenderOnce, SharedString, Stateful, StatefulInteractiveElement, Styled,
+    TestSupportExt, Window, div, px,
 };
 
 use super::{ClickHandler, keyboard_focusable};
@@ -24,6 +27,7 @@ pub struct SidebarItem {
     trailing: Option<SharedString>,
     selected: bool,
     on_click: Option<ClickHandler>,
+    focus: Option<FocusHandle>,
 }
 
 impl SidebarItem {
@@ -35,6 +39,7 @@ impl SidebarItem {
             trailing: None,
             selected: false,
             on_click: None,
+            focus: None,
         }
     }
 
@@ -63,6 +68,16 @@ impl SidebarItem {
     }
 }
 
+impl SidebarItem {
+    /// Tracks `handle`, so focusing it focuses the row. A row that is a
+    /// keyboard stop needs a handle that is a tab stop.
+    #[must_use]
+    pub fn track_focus(mut self, handle: FocusHandle) -> Self {
+        self.focus = Some(handle);
+        self
+    }
+}
+
 /// Resolved colors for a row.
 #[derive(Debug, Clone, Copy, PartialEq)]
 pub struct RowStyle {
@@ -113,14 +128,9 @@ impl RenderOnce for SidebarItem {
                 self.trailing
                     .map(|t| div().text_color(colors.get(Role::TextMuted)).child(t)),
             );
-        if let Some(background) = style.background {
-            row = row.bg(background);
-        }
-        if let Some(border) = style.border {
-            row = row.border_color(border);
-        }
-        if let Some(hover) = style.hover {
-            row = row.hover(move |s| s.bg(hover));
+        row = paint(row, style);
+        if let Some(focus) = &self.focus {
+            row = row.track_focus(focus);
         }
         if let Some(handler) = self.on_click {
             row = keyboard_focusable(row, &colors)
@@ -130,6 +140,20 @@ impl RenderOnce for SidebarItem {
     }
 }
 
+/// Applies a row's resolved colors.
+fn paint(mut row: Stateful<Div>, style: RowStyle) -> Stateful<Div> {
+    if let Some(background) = style.background {
+        row = row.bg(background);
+    }
+    if let Some(border) = style.border {
+        row = row.border_color(border);
+    }
+    if let Some(hover) = style.hover {
+        row = row.hover(move |s| s.bg(hover));
+    }
+    row
+}
+
 #[cfg(test)]
 mod tests {
     use super::row_style;
```

- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run --workspace`
Expected: every test passes, including the following:
- the reopen, first-line, only-tab and `Forgetful` tests in `tests/app/tabs.rs`;
- the table tests in `tests/app/palette.rs`;
- `a_row_with_a_tracked_handle_takes_focus_from_code`.

Run: `cargo mutants -p quarry -f crates/app/src/model/tabs.rs -f crates/app/src/model/closed_tabs.rs -f crates/app/src/model/schema_tree.rs`
Expected: 0 missed (59 caught, 6 unviable in the prototype). Then `rm -rf mutants.out mutants.out.old`.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/app/src/model/closed_tabs.rs crates/app/src/model/schema_tree.rs crates/app/src/model/tabs.rs crates/app/src/views/palette/closed.rs crates/app/src/views/palette/mod.rs crates/app/src/views/palette/tables.rs crates/app/src/views/schema.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/mod.rs crates/app/src/views/workspace/tabs.rs crates/app/tests/app/palette.rs crates/app/tests/app/tabs.rs crates/ui-kit/src/components/sidebar_item.rs crates/ui-kit/tests/keyboard.rs
git commit -m "feat(app): list tables and recently closed tabs in the palette"
```

---

### Task 9: ADR 0011, the README, and a note in the parent spec

Record the decisions in ADR 0011 "Command palette and copying". Move the README's status to "Phase 1 complete", and note in §4.2 of the parent spec what 1E built of the palette and copying.

**Files:**
- Modify: `README.md`
- Create: `docs/adr/0011-command-palette-and-copying.md`
- Modify: `docs/superpowers/specs/2026-11-04-quarry-design.md`

**Interfaces:**
- Consumes: everything above.
- Produces: documentation only.

- [ ] **Step 1: Implement**

Modify `README.md`:
```diff
@@ -2,7 +2,7 @@
 
 A native macOS SQL client for learning SQL by hand and for real work, built in Rust with GPUI.
 
-> **Status:** Phase 1D. You can write and run SQL against a built-in sample database in query tabs that survive restarts, run whole scripts, and explain a statement's plan. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).
+> **Status:** Phase 1 complete. You can write and run SQL against a built-in sample database in query tabs that survive restarts, run whole scripts, explain a statement's plan, copy results as TSV, CSV or JSON, and jump to tabs, closed tabs, tables and commands with ⌘K. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).
 
 - Built-in SQLite playground with story-driven SQL challenge tracks
 - PostgreSQL and MySQL connections for everyday work
```

Create `docs/adr/0011-command-palette-and-copying.md`:
```markdown
# ADR 0011: Command palette and copying

- **Status:** Accepted
- **Date:** 2026-11-14

## Context
Phase 1E completes Phase 1 ("Shell + design system") with the ⌘K command palette and copying results. Both are extension points the parent spec names (§3.3): later phases add palette sources (challenges, tracks, connections, saved queries, settings) and export formats (exporting to a file, Phase 3). Phase 1E also closes the leftovers from Phase 1D: saves that could land out of order, two copies of the one-off reply poll, a test helper that hid timeouts, and view tests that were never written.

## Decision
- **Exporters live in the engine.** `Exporter` formats rows with a header and single values; `ExporterRegistry::standard()` holds TSV, CSV and JSON, one module each. They format full `Value`s, never the grid's cut-off cell text. TSV quotes a field CSV-style only when a tab, line break or quote would break the line, so spreadsheets paste it as typed. JSON keeps column order and numbers repeated names, as in `name (2)`; a float that is not finite becomes `null`, and bytes become their `\x…` hex as a string.
- **What a copy takes is a model.** `model::copy` turns the grid's selection (none, a row, a column or a cell) into rows or one value, and writes the status-bar note, including "Copied 100,000 of 250,000 rows as TSV" when the result was cut off. Formatting runs on the background executor; the clipboard is written on the main thread. ⌘C copies as TSV only while the grid has focus, so the editor keeps its own ⌘C. Edit → Copy as … and the palette copy the active tab's shown result from anywhere.
- **Palette sources are a registry.** `PaletteSource` lists items and acts on a picked one; `PaletteSourceRegistry::standard()` holds Open tabs, Recently closed, Tables & views and Commands, in that order. The dialog wraps gpui-component's `Command` with its own filtering off: `model::palette::filter` ranks a prefix, then a word start, then anywhere, keeps each source's order for ties, and shows at most 50 results. Items are collected when the palette opens. Picking closes the dialog first, so focus is back in the workspace when the source acts. The palette navigates and runs commands; it never offers SQL.
- **One command table.** `shell::actions::commands()` lists every command as (label, action, menu). The menus are built from it, the bindings test checks it, and the Commands source lists it, so a new command appears in all three. The app menu (Quit) is left out of the palette. ⌘K is bound in the workspace, in its editor, and in the palette's own key context, where it closes the palette again.
- **Test windows sit in `Root`.** The headless test helper wraps the workspace in gpui-kit's `Root`, as the real window does, so dialogs open in tests.
- **Closed tabs are drafts.** `Drafts::closed` holds the last 10 closed tabs, newest first, in schema v2's `closed_tabs` table, saved in the same transaction as the open tabs. `DraftStore` does not change. ⌘W remembers a tab, including the only one; blank tabs are not remembered, and the same title and text moves to the front. A closed tab's id is not kept: it reopens as a new tab with a new id.
- **Revealing a table.** The schema tree scrolls on its own and gives each table row a focus handle (`SidebarItem::track_focus`), so picking a table opens the sidebar, expands the table, scrolls to it and focuses its row.
- **Saves land in order.** Every snapshot takes a generation from `SaveOrder`, and every write holds one lock and is skipped when a newer generation has already been written (ADR 0010).
- **One reply poll.** `views::poll::poll_reply` polls a one-off reply every 33 ms; the schema read and Explain both use it. A second ⌘E drops the pending plan's task, and with it its channel.

## Consequences
- A new palette source or export format is one module and one registration line, with no change to the dialog or the copy path.
- Copying 100,000 rows clones them on the main thread before formatting them in the background; this takes a moment but never blocks on the formatting.
- The palette's items are a snapshot: a tab closed while it is open is ignored when picked, and a dropped table re-reads the schema and focuses nothing.
- Reopened tabs get new ids, so nothing may rely on a closed tab's id.
```

Modify `docs/superpowers/specs/2026-11-04-quarry-design.md`:
```diff
@@ -165,6 +165,7 @@ Reference mockups are in `docs/design/mockups/` (`visual-direction.html`, `light
 **Command palette (⌘K)**
 - Jump to any track, challenge, table, tab, saved query, connection or setting.
 - It's for navigating only and never suggests SQL.
+- Phase 1E built it with open tabs, recently closed tabs (the last 10, kept across restarts), tables and views, and commands; later phases add the other sources. ⌘C in the grid copies as TSV, and Edit → Copy as … copies as TSV, CSV or JSON (ADR 0011).
 
 ### 4.3 Feedback on Check answer
 
```

- [ ] **Step 2: Run the tests and every gate**

Run: `just check`
Expected: exit 0 (fmt, lint, quality, deny, machete, typos, complexity, coverage). The prototype passed with 382 tests.

Run: `git diff main > target/1e.diff && just mutants target/1e.diff`
Expected: the mutation gate passes with 0 missed. Then `rm -rf mutants.out mutants.out.old target/1e.diff`.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/adr/0011-command-palette-and-copying.md docs/superpowers/specs/2026-11-04-quarry-design.md
git commit -m "docs: record the command palette and copying (ADR 0011)"
```

---

## After the last task

- [ ] **Visual review.** Run `cargo run -p quarry` and check the following with the user:
  - The palette in the dark and light themes, with an empty query, a filtered query and "No matches".
  - Picking from each source.
  - ⌘C in the grid and the status-bar note.
  - A table revealed from a closed sidebar.

  The spec makes this review a condition of the PR.
- [ ] **Open the PR** only after the review, using `.github/pull_request_template.md`. Merge only with the user's explicit go-ahead.
