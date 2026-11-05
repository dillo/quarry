# ADR 0002: SQLite (rusqlite, bundled) as the built-in database

- **Status:** Accepted
- **Date:** 2026-11-05

## Context
The challenges need a database that ships inside the app, resets instantly and works offline.

## Decision
- Use bundled SQLite.
- Challenge sandboxes are built from track templates and loaded into memory with the backup API.

## Consequences
- Nothing to install, and resets take about 1 ms.
- The SQL dialect differs slightly from Postgres, so challenges avoid dialect-specific features or say when they use one.
