# Cellar — Confirmed & Suspected Bugs

> **Status update (2026-06-10):** B1, B2, B13 fixed in [#58](https://github.com/MRL-00/cellar/pull/58); B4 fixed in [#57](https://github.com/MRL-00/cellar/pull/57); B9 fixed in [#60](https://github.com/MRL-00/cellar/pull/60) — all merged. Still open: B3, B5, B6, B7, B8, B10, B11, B12.

Companion to [improvement-plan.md](improvement-plan.md). The roadmap docs cover missing features; this is a **correctness bug hunt** (the user's original ask included "bugs that need to be fixed"). Found via code review of the Rust drivers, shared state, the diff engine, the data grid, and the frontend stores.

**Confidence levels:** ✅ _Verified_ = I read the exact code and the bug is unambiguous. 🟡 _Likely_ = pointed at real code with a plausible failure path, but depends on library behavior or a path I didn't execute — confirm with a test before fixing.

---

## High severity

### ✅ B1 — Pressing **Escape** in a cell editor commits the edit instead of cancelling
`packages/data-grid/src/Cell.tsx:85-89`
```ts
onKeyDown={(e) => {
  if (e.key === "Enter") onCommit(v);
  if (e.key === "Escape") onCancel();   // cancels…
}}
onBlur={() => onCommit(v)}              // …then blur immediately commits
```
Pressing Escape calls `onCancel()`, which unmounts the input → the input blurs → `onBlur` fires `onCommit(v)`. **The cancellation is overridden and the edit is written.** A user who opens a cell, types a wrong value, and hits Escape to back out has just committed the wrong value to pending changes.
**Fix:** guard blur with a ref set on cancel/commit (e.g. `committedRef`), or use `onKeyDown` `preventDefault` + a "settled" flag so blur is a no-op after Enter/Escape.

### ✅ B2 — Opening a NULL cell and closing it records a spurious `NULL → ''` edit
`packages/data-grid/src/Cell.tsx:49`
```ts
const [v, setV] = useState<string>(value == null ? "" : String(value));
```
A database NULL initializes the editor to `""`. Double-click a NULL cell, change nothing, press Enter (or blur) → `onCommit("")`. Since `null !== ""`, the grid records `{from: null, to: ""}` — a phantom change marking an untouched NULL cell as edited. The pending bar shows "1 update" and the commit modal emits `SET col = ''` on a row the user never meant to touch.
**Fix:** track whether the value actually changed; treat an unedited editor as a no-op; distinguish "" (intentional empty) from NULL (untouched) — likely needs an explicit "set to NULL" affordance rather than overloading empty string.

### ✅ B3 — SQL Server `Contains` filter doesn't escape LIKE metacharacters → wrong rows
`crates/cellar-drivers/sqlserver/src/table_browse.rs:185-193`
```rust
Ok(format!("{ident} LIKE {}", quote_literal(&format!("%{value}%"))))
```
`quote_literal` escapes single quotes but **not** `%`, `_`, or `[`. A `Contains` filter for `50%` becomes `LIKE N'%50%%'` (matches far more than intended); `[abc]` becomes a T-SQL character class. Not SQL injection (the value is still quoted), but **silently returns the wrong result set**. The Postgres path is safe — it uses `push_bind` (parameterized). Fix: escape `%`/`_`/`[` and add `ESCAPE '\'`.

### ✅ B4 — `ConnectionRegistry.save`/`delete` mutate in-memory state *before* persisting → divergence on I/O failure
`apps/desktop/src-tauri/src/state.rs:89-93` (save) and `:97-106` (delete)
```rust
inner.configs.insert(config.id.clone(), config.clone());
persist(&inner.configs).await?;   // if this Errs, memory already changed
```
If `persist` fails (disk full, perms, missing home dir), the function returns `Err` but memory is already mutated. For `delete`, the connection is also already closed and removed — the UI reports failure, yet the connection is gone until restart, and reappears from disk after restart. **Fix:** persist to a temp file and rename first, then mutate memory only on success (persist-then-commit ordering).

### 🟡 B5 — Postgres `OID` columns decoded as `i64` → likely decode error on every catalog query
`crates/cellar-drivers/postgres/src/decode.rs:42-45`
```rust
"OID" => row.try_get::<i64, _>(ordinal)...
```
Postgres `oid` is a 4-byte unsigned type (wire OID 26), distinct from `int8`. sqlx 0.8's `i64` decoder type-checks against `INT8`, so `try_get::<i64>` on a real `oid` column should return a decode error, failing the whole row. Triggered by e.g. `SELECT oid, relname FROM pg_class`. **Verify with a quick test against a catalog query; fix is `try_get::<u32, _>`.** (Same file: 🟡 `TIMETZ` decoded as `NaiveTime` at `:84-87` — `timetz` is wire OID 1266 and carries an offset `NaiveTime` can't accept; likely decode error + drops the zone.)

### ✅ B6 — `queryMessages` and `notices` are never cleared when a tab closes → leak + message eviction
`apps/desktop/src/state/tabs.ts` (closeTab `:229`, closeOtherTabs `:252`, closeTabsToRight `:277`, closeConnectionTabs `:304`)
**Verified by grep:** `clearForTab` is defined at `state/queryMessages.ts:12,44` and is **never called anywhere** in `apps/desktop/src`. All four close paths call `clearTabResults` and `dropTabScopedState` (which clean `tableChanges`/`refreshKeys`/results) but none call `clearForTab` or clear `useNotices.byScope`. Orphaned messages from closed tabs accumulate; with the global `MAX_MESSAGES=300` cap, messages from *still-open* tabs get evicted by ghosts from dead ones. **Fix:** call `clearForTab` / notice-scope cleanup in every close path.

## Medium severity

### ✅ B7 — Sticky `dirty` flag: a fully-reverted query tab stays marked unsaved
`apps/desktop/src/state/tabs.ts:215` — `dirty: sql !== t.sql ? true : t.dirty`. **Verified:** `dirty` only clears in `markQueryRun:221`. It compares against the *previous* value each keystroke, never a saved baseline, so once set it never returns to `false` without a run. Type a char then delete it and the tab keeps its unsaved-dot until you run a query. Fix: compare against a stored baseline/last-run SQL.

### 🟡 B8 — EXPLAIN plan result has no stale-guard → lands on the wrong tab
`apps/desktop/src/components/BottomPlanPanel.tsx:57-78` — `loadPlan` awaits `commands.explainQuery(...)` then `setPlan(next)` with no mounted/generation check. Click Explain on a slow query, switch tabs → the resolved plan overwrites the new tab's panel. Fix: generation counter or `cancelled` ref (same pattern needed in the table-load and query paths).

### ✅ B9 — SQL Server `execute_query` drains the full result after the cap (`continue` vs `break`)
`crates/cellar-drivers/sqlserver/src/query.rs:62-66` — **Verified:** once `rows.len() >= max_rows` it sets `truncated` and `continue`s. The `continue` sits *before* `rows.push`, so it correctly skips *decoding* the extra rows — but it still iterates the stream to completion, so **every remaining row is transferred over the network**. A 1M-row query with cap 500 still pulls 1M rows over the wire (just doesn't decode 999,500 of them). Fix: `break`. (Ties into the Tier-0 pagination work.)

### 🟡 B10 — No-PK tables key pending edits by row position → wrong row on concurrent change
`apps/desktop/src/hooks/useTableData.ts:299` — `rowIdFor` returns `row:${index}` when there's no primary key. Pending edits survive sort/filter within a page (row objects keep their id), but across a page reload, position `5` may be a *different* row if another session modified the table; the commit modal applies the change without re-verifying by PK. Affects views/log/temp tables. Fix: hash the full row contents as the identity for no-PK tables, or block editing without a PK and say so.

## Low severity / polish

- 🟡 **B11 — Postgres connection URL not percent-encoded.** `connect.rs:121-128` builds `postgres://{user}@{host}:{port}/{db}` by string interpolation; a `user` containing `@` or a `database` with `/` misroutes or fails with a confusing parse error. Fix: use `PgConnectOptions::new()` builder (`.username()/.host()/.database()`) instead of a URL.
- ✅ **B12 — Closing the active tab jumps to the *last* tab, not the adjacent one.** `tabs.ts:234` picks `tabs[tabs.length-1]`. Close tab 3 of 10 → you land on tab 10. Fix: select the neighbor (left, or right if leftmost).
- 🟡 **B13 — `CellEditor` double-commits on Enter.** Enter fires `onCommit` then blur fires it again — idempotent but an extra Zustand write + grid re-render per confirm (visible flicker on large grids). Same blur-after-unmount mechanism as B1.

---

## Suggested triage order
1. **B1 + B2** (data-grid edit correctness — Escape commits, NULL phantom edits). Same file, same blur mechanism; fix together. Highest user-trust impact.
2. **B3** (SQL Server filter correctness) and **B4** (state divergence on save/delete) — both silent-wrong-result/corruption class.
3. **B5/B9** fold into the driver work; **B6/B7/B8** into the frontend store/effect cleanup pass.
4. **B10** gate or fix before promoting no-PK table editing.

_Verified (✅): B1, B2, B3, B4, B6, B7, B9, B12. Likely / verify-with-test (🟡): B5, B8, B10, B11, B13._

_B5 (Postgres OID/TIMETZ decode) needs a live Postgres catalog query to confirm — couldn't runtime-verify statically. B8/B10/B13 are race/edge-condition bugs whose code paths are confirmed present but need a runtime repro to demonstrate the failure._
