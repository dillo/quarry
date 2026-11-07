# ADR 0007: Synchronous drivers on a session thread

- **Status:** Accepted
- **Date:** 2026-11-06

## Context
Spec §3.4 sketched an `async` `Driver` trait. SQLite, the only Phase 1 driver, is synchronous. GPUI has its own executor, so tying drivers to Tokio would push a runtime into the engine's API.

## Decision
`Driver` is synchronous, and a `Session` runs it on a dedicated thread, streaming `RunEvent`s over a bounded `async-channel`. UI code can `await` them on any executor.

Cancellation is per run:
- Each run has its own `cancelled` flag, and the driver exposes a `Canceller` flag that SQLite's progress handler checks every 1,000 VM steps.
- The session tracks the executing run in a `Mutex<u64>` (`current_run`, 0 when idle). Cancel, run start, each statement start (reset the driver flag, then check the run flag), run end (reset) and schema reads (reset) all happen under that lock. A cancel therefore only ever affects the run it belongs to, and a stale cancel cannot leak into the next run or statement.
- Dropping a `RunHandle` cancels its run.
- Dropping the `Session` sets a `closed` flag under the lock and interrupts the current statement, so later statements and queued runs fail with `Cancelled`. A schema read queued before the drop still completes, which is bounded work.
- The lock guards only flag transitions. It is never held while a statement runs or an event is sent.

Backpressure: the event channel is bounded, so a reader that neither reads nor drops its handle stalls the session. Consumers must read until `Done` or drop the handle.

Phase 3's async drivers will run their own runtime inside the session thread.

YAGNI cuts:
- The destructive-statement classifier moves to Phase 3 (production guardrails).
- Table row counts are deferred.
- `Value` has only the variants SQLite produces.
- Sample data lives in `app` (1C), not in the engine.

## Consequences
- The UI never blocks, and a slow reader throttles the worker.
- One running query per session, which is fine for one tab per session.
- Async drivers will need a small runtime inside the session thread.
- Consumers must drain or drop every `RunHandle`.
