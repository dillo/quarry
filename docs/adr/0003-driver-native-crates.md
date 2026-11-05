# ADR 0003: tokio-postgres and mysql_async over sqlx

- **Status:** Accepted
- **Date:** 2026-11-05

## Context
⌘. must cancel a running query on the server.

## Decision
- Use `tokio-postgres`, which has a native cancel request, and `mysql_async`, which can cancel with `KILL QUERY` on a second connection.
- Each is wrapped behind `engine::Driver`.

## Consequences
Real cancellation and row streaming, at the cost of two driver implementations instead of one.
