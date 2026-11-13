# Phase 1E: the ⌘K palette and copying results

Status: approved in brainstorming on 2026-11-13. Parent spec: `2026-11-04-quarry-design.md` (§3.3, §4.2, §9). Previous phase: `2026-11-09-phase-1d-tabs-runs-design.md`.

## 1. Scope

Phase 1E completes Phase 1 ("Shell + design system").

**In 1E:**
- **The ⌘K command palette.** It has four sources:
  - Open tabs
  - Recently closed (the last 10 closed tabs, kept across restarts)
  - Tables & views
  - Commands
- **Copying results:**
  - ⌘C in the grid copies as TSV.
  - Edit → Copy as TSV, Copy as CSV and Copy as JSON. All three are also ⌘K commands.
- **Leftovers from 1D:**
  - a guard so saves always land in order;
  - one shared helper for polling a one-off reply;
  - `settle()` reports a timeout;
  - the view tests 1D left out.

**Not in 1E:**
- Exporting to a file (Phase 3).
- Selecting several rows at once.
- Sources for challenges, tracks, connections, saved queries or settings (later phases).
- Renaming tabs.
- Anything that suggests SQL. The palette navigates and runs commands only.

## 2. Global constraints

These come from the parent spec and bind every task.

- **No completion or assistance of any kind.**
  - The editor gets no completion, hover, definition or code-action providers, no inline suggestions and no hints.
  - The palette never offers SQL.
- **Function size:** at most 15 statements and 40 lines per function. Each unit has a single responsibility.
- **Extension points:** new kinds of things go through traits and registries (§3.3 of the parent spec): `Exporter`/`ExporterRegistry` and `PaletteSource`/`PaletteSourceRegistry`. There are no `match` statements on a registry-owned enum outside its module.
- **Dependency direction:**
  - `engine` depends on no workspace crate.
  - `ui-kit` depends only on GPUI.
  - `store` depends on no workspace crate apart from `engine`, and 1E needs none.
  - `xtask check-deps` enforces all three.
- **Coverage and mutation testing:**
  - `engine` and `store` need at least 95% lines and 90% regions.
  - The app needs at least 80% lines and 70% regions.
  - Every file in `crates/app/src/model` stays at 100%.
  - Mutation testing finds 0 missed in `engine`, `store`, `crates/app/src/model` and `crates/app/src/sample`.
  - Views are covered by GPUI tests.
- **No `unwrap`/`expect`/`panic!`/`unreachable!` outside `#[test]` functions**, test helpers included.
- **Polling:** GPUI code never awaits a session channel. It polls every `POLL` (33 ms). Background work uses GPUI's executor, so tests stay deterministic.
- **Contrast:** every new color goes through a `Role` and is covered by `tests/contrast.rs`.
- **Exact versions:** gpui-kit =0.9.0 and rusqlite 0.41.1 (the workspace dependency).
- **Merging:** only with the user's explicit go-ahead.

## 3. Architecture

### 3.1 `quarry-engine`: exporters

```rust
pub trait Exporter: Send + Sync {
    fn id(&self) -> &'static str;      // "tsv", "csv", "json"
    fn title(&self) -> &'static str;   // "TSV", "CSV", "JSON"
    fn rows(&self, columns: &[Column], rows: &[Row]) -> String;  // with a header
    fn value(&self, value: &Value) -> String;                    // a single cell
}

pub struct ExporterRegistry { /* Vec<Box<dyn Exporter>> */ }
impl ExporterRegistry {
    pub fn standard() -> Self;                      // Tsv, Csv, Json, in that order
    pub fn get(&self, id: &str) -> Option<&dyn Exporter>;
}
```

- `export.rs` holds the trait and the registry. Each format lives in its own module (`export/tsv.rs`, `csv.rs`, `json.rs`). Adding a format means one new module and one line in `standard()`.
- Exporters format full `Value`s. They never use the grid's cut-off cell text.
- `rows` with a single column and rows of one value each is how a column copy is made.

| | TSV | CSV | JSON |
|---|---|---|---|
| Rows | One line per row, fields separated by tabs, lines ending in `\n` | RFC 4180: comma-separated, lines ending in `\r\n` | An array of objects keyed by column name, in column order, pretty-printed with two-space indents |
| `NULL` | empty field | empty field | `null` |
| Int, Float | the engine's text | the engine's text | JSON numbers. A non-finite float becomes `null`. |
| Text | as is; quoted CSV-style if it contains a tab, `\n`, `\r` or `"` | quoted if it contains a comma, `\n`, `\r` or `"` | a JSON string |
| Bytes | `\x` plus lowercase hex | `\x` plus lowercase hex | the `\x…` hex as a string |
| Duplicate column names | as is | as is | the second becomes `name (2)`, the third `name (3)`, and so on |
| `value()` (one cell) | the raw text, unquoted | the field, quoted when needed | the JSON value |
| No rows | the header line only | the header line only | `[]` |

### 3.2 `quarry-store`: closed tabs (schema v2)

- `Drafts` gains `pub closed: Vec<Draft>`: newest first, at most 10. `save` writes it in the same transaction as the tabs. **The `DraftStore` trait does not change**, and neither does `Forgetful`.
- **Migration v2:** `closed_tabs(position INTEGER NOT NULL, title TEXT NOT NULL, text TEXT NOT NULL)`.
  - A v1 file is upgraded in place, in one transaction.
  - A file with a newer `user_version` is refused as before (`TooNew`) and is never modified.
- `load` of a v1 file, after migrating, returns no closed tabs.

### 3.3 App: `model/` (GPUI-free, 100% coverage, mutation-tested)

| Module | Responsibility |
|---|---|
| `palette.rs` | `PaletteItem { source: &'static str, key: String, label: String, detail: String }`. `filter(items, query) -> Vec<&PaletteItem>`:<br>• matching is case-insensitive and on the label;<br>• rank: prefix, then a word start, then anywhere;<br>• ties keep the source's order, and the source groups keep their order;<br>• at most `PALETTE_LIMIT` = 50 results;<br>• an empty query returns every item, up to the limit. |
| `copy.rs` | Turns the grid's selection into what gets copied. The selection is `None`, `Row(r)`, `Column(c)` or `Cell(r, c)`, and the result is `Copy::Rows { columns, rows }` or `Copy::Value`. `None` means every kept row. An index out of range means nothing. Also makes the note: "Copied 12 rows as CSV", "Copied 100,000 of 250,000 rows as TSV", "Copied 1 value as JSON" or "Nothing to copy". |
| `closed_tabs.rs` | `remember(&mut Vec<Draft>, Draft)`:<br>• ignores empty or whitespace-only text;<br>• moves an identical title and text to the front instead of duplicating it;<br>• caps the list at `CLOSED_LIMIT` = 10.<br>`take(&mut Vec<Draft>, index) -> Option<Draft>` removes one entry. |
| `save_order.rs` | `SaveOrder`: `next() -> u64` hands out rising generations, and `commit(generation) -> bool` returns false for a generation older than the last committed one. |

### 3.4 App: `views/`

- **`palette/mod.rs`:** the `PaletteSource` trait and `PaletteSourceRegistry`.

  ```rust
  pub trait PaletteSource {
      fn id(&self) -> &'static str;
      fn title(&self) -> &'static str;                        // the group heading
      fn items(&self, workspace: &Workspace, cx: &App) -> Vec<PaletteItem>;
      fn pick(&self, key: &str, workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>);
  }
  ```

  `PaletteSourceRegistry::standard()` registers four sources, in this order: `tabs`, `closed`, `tables` and `commands`, one module each. Adding a source means one new module and one registration line.
- **`palette/dialog.rs`:** the palette dialog. It wraps gpui-component's `Command` with `filterable(false)`. Each `on_query` re-runs `model::palette::filter` over the items collected when the palette opened, and `on_confirm` closes the dialog, then calls the source's `pick`. It opens through gpui-kit's `Root` dialog layer, which every window already has.
- **`views/poll.rs`:** `poll_reply`, which waits for a one-off reply. It polls a `Receiver<Result<T, EngineError>>` every `POLL` with `drain::reply` and returns the answer. A channel that closes without a reply gives `WorkerGone`. The schema read and Explain both use it.
- **Copy in `query_tab`:**
  - `QueryTab::copy(exporter, cx)` reads the grid's selection and the shown statement's output, then builds a `Copy` with `model::copy`.
  - It formats on the background executor and writes the clipboard on the main thread.
  - It emits `QueryTabEvent::Copied(note)`.
- **`workspace/`:**
  - shows the copy note in the status bar for `NOTE_TIME` = 3 s;
  - keeps the closed-tab list in its drafts snapshot;
  - takes every save, including those at close and quit, through `SaveOrder` and a single save lock.

### 3.5 App: `shell/`

- **New actions and key bindings:**

  | Action | Key | Context | Notes |
  |---|---|---|---|
  | `OpenPalette` | ⌘K | `Workspace` and `Workspace > Input` | |
  | `CopyTsv` | ⌘C | the grid | The editor keeps its own ⌘C. |
  | `CopyCsv` | none | | |
  | `CopyJson` | none | | |

- **Menus:**
  - A new Edit menu with Copy as TSV, Copy as CSV and Copy as JSON.
  - Open Palette goes in View.
- **One command table:** `actions.rs` holds a single list of `(label, action, menu)`. It is the source for the menus, the bindings test and the Commands palette source, so a new command appears in all three. Quit is left out of the palette.

## 4. Behaviour

### 4.1 The palette

**Opening and closing**
- ⌘K opens a centered dialog with a search field. It works while the editor has focus.
- Esc, a click outside it, or ⌘K again closes it without doing anything.
- Focus goes back to where it was before.

**With an empty query**, the groups appear in this order:

| Group | Each item shows | Order |
|---|---|---|
| **Open tabs** | The title. The active tab's detail reads "current". | Tab order |
| **Recently closed** | The title, with the first non-blank line of its text as detail | Newest first |
| **Tables & views** | The name, with "table N columns" or "view N columns" as detail | Sidebar order |
| **Commands** | The label, with its shortcut on the right | Menu order |

**Typing** filters every group using the `palette` model's ranking. Groups with no matches are hidden. If nothing matches, the dialog shows "No matches".

**Picking an item** (↵ or a click) closes the palette, then:

| Source | Effect |
|---|---|
| **Open tab** | Activates that tab, exactly like clicking it in the tab strip, including the immediate save. |
| **Recently closed** | Reopens it as a new tab after the active tab, with its title and text. The entry is removed from the list, and the drafts save straight away. |
| **Table or view** | Opens the sidebar if it is closed, expands that table, scrolls it into view and focuses its row. |
| **Command** | Dispatches its action, the same as its shortcut or menu item. |

**Remembering closed tabs:**
- ⌘W remembers the closed tab through `closed_tabs::remember`.
- Closing the only tab remembers it too, before the fresh "Query N" replaces it.

### 4.2 Copying

**What gets copied**

| Grid selection | Copied |
|---|---|
| Nothing | The header and every kept row of the shown result tab |
| A row | The header and that row |
| A column | That column's header and its values |
| A cell | The value alone |

- ⌘C works only while the grid has focus.
- Edit → Copy as … and the palette commands copy the active tab's shown result from anywhere.
- The status bar shows the note for 3 s:
  - "Copied N rows as FORMAT"
  - "Copied 1 value as FORMAT"
  - "Copied 100,000 of 250,000 rows as TSV", when the result was cut off at `ROW_LIMIT`
  - "Nothing to copy", when there are no rows, the statement returns no rows, or nothing has run
- A newer note replaces an older one.

### 4.3 Saving in order

- Every snapshot gets a generation from `SaveOrder::next` when it is taken. This covers background saves, the save when the window closes and the save at quit.
- A save writes while holding one lock. It is skipped if `commit` says a newer generation has already been written.
- So an older snapshot can never land after a newer one. ADR 0010 is updated to say so.

## 5. Errors

| Situation | Behavior |
|---|---|
| Nothing to copy | The status bar shows "Nothing to copy". |
| A picked table no longer exists | The schema is re-read and nothing is focused. The item is gone the next time ⌘K opens. |
| A picked tab was closed while the palette was open | Nothing happens. |
| Reopening a closed tab while drafts aren't being saved (`Forgetful`) | It works from the in-memory list for this session. |
| A v1 drafts file | It is migrated to v2 in place. Its tabs are kept and its closed list starts empty. |
| A drafts file from a newer Quarry | Unchanged from 1D: refused, never modified, and the status-bar warning shows. |

## 6. Testing

- **Engine.** Each exporter is tested for:
  - `NULL`, numbers, non-finite floats, bytes and text that needs quoting;
  - duplicate column names (JSON);
  - single cells, no rows and the header line.
- **Store:**
  - a v1 file migrates to v2 and keeps its tabs;
  - closed tabs survive a save and load round trip;
  - an empty closed list round-trips;
  - `TooNew` still leaves the file byte-identical.
- **Models.** Every rule in §3.3 has unit tests:
  - ranking order, ties, hidden empty groups, the 50-item limit and case-insensitivity;
  - each selection kind, indexes out of range, and each note;
  - closed tabs: dedupe, the cap and ignoring blank text;
  - `SaveOrder` generations.
- **Views (GPUI tests):**
  - ⌘K opens the palette and Esc closes it, with focus restored.
  - Picking from each source: a tab switches, a closed tab reopens, a table is revealed and focused, and a command runs.
  - Filtering hides empty groups and "No matches" appears.
  - ⌘C in the grid puts TSV on the test clipboard. Copy as CSV and Copy as JSON from the menu do the same. The note appears, and "Nothing to copy" shows when there is nothing.
  - Saves land in order: a delayed older snapshot is skipped.
  - The tests 1D left out:
    - ⌘↵ from Messages returns to Results.
    - A Run all with one row-returning statement shows no result-tab strip.
    - The failure banner shows over Messages.
    - A second ⌘E while a plan is pending replaces the first.
    - ⌘W activates the right-hand neighbour.
- **`settle()`** returns whether its condition held, and every caller asserts it.
- **The editor** still has no completion, hover, definition or code-action providers.
- **A visual review** by the user before the PR.

## 7. Tasks

1. Engine: `Exporter`, `ExporterRegistry`, and TSV, CSV and JSON.
2. Store: schema v2 and `Drafts::closed`.
3. Models: `palette`, `copy`, `closed_tabs`, `save_order`.
4. `poll_reply`, `settle()` reporting timeouts, and the tests 1D left out.
5. Saving in order: `SaveOrder` and the save lock, with ADR 0010 updated.
6. Copying: `QueryTab::copy`, the status-bar note, the Edit menu and ⌘C.
7. The palette: `PaletteSource`, the registry, the dialog, and the Open tabs and Commands sources, with ⌘K and the single command table.
8. Palette sources: Tables & views, and Recently closed (remembering a tab on ⌘W, reopening it).
9. ADR 0011 "Command palette and copying", the README, and a note in §4.2 of the parent spec.

As in 1C and 1D, a verified prototype is built first, and the plan is generated from it.
