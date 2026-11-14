# ADR 0011: Command palette and copying

- **Status:** Accepted
- **Date:** 2026-11-14

## Context
Phase 1E completes Phase 1 ("Shell + design system") with the ⌘K command palette and copying results. Both are extension points the parent spec names (§3.3): later phases add palette sources (challenges, tracks, connections, saved queries, settings) and export formats (exporting to a file, Phase 3). Phase 1E also closes the leftovers from Phase 1D: saves that could land out of order, two copies of the one-off reply poll, a test helper that hid timeouts, and view tests that were never written.

## Decision
- **Exporters live in the engine.** `Exporter` formats rows with a header and single values; `ExporterRegistry::standard()` holds TSV, CSV and JSON, one module each. They format full `Value`s, never the grid's cut-off cell text. TSV quotes a field CSV-style only when a tab, line break or quote would break the line, so spreadsheets paste it as typed. JSON keeps column order and numbers repeated names, as in `name (2)`; a float that is not finite becomes `null`, and bytes become their `\x…` hex as a string.
- **What a copy takes is a model.** `model::copy` turns the grid's selection (none, a row, a column or a cell) into rows or one value, and writes the status-bar note, including "Copied 100,000 of 250,000 rows as TSV" when the result was cut off. Formatting runs on the background executor; the clipboard is written on the main thread. The workspace keeps the one copy still formatting, so a newer copy, even one with nothing to copy, cancels it. ⌘C copies as TSV only while the grid has focus, so the editor keeps its own ⌘C. Edit → Copy as … and the palette copy the active tab's shown result from anywhere.
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
