# Wishlist Items — Implementation-Ready Breakdowns

Companion to [improvement-plan.md](improvement-plan.md) (Wishlist section) and [improvement-plan-top5.md](improvement-plan-top5.md). User-requested features, grounded in the actual code as of 2026-06-09.

Key fact that shapes all three: **the transactional commit engine already exists.** `cellar-diff` (`crates/cellar-diff/src/lib.rs`) models:
```
TableChangeRequest { schema, table, primary_key: Vec<String>, changes: Vec<RowChange> }
RowChange::Update { row_id, keys, assignments }  // match on keys, set assignments
RowChange::Insert { row_id, values }
RowChange::Delete { row_id, keys }
build_postgres_plan(req) -> CommitPlan { statements, preview { sql, expected_rows } }
```
and `commit_table_changes` (`postgres/src/query.rs:86`) runs the plan in a single `BEGIN`/`COMMIT` with affected-row validation and rollback-on-mismatch. The grid edit path already uses it. **The wishlist items below mostly feed new front-ends into this same engine** rather than building new backends.

---

## W1. CSV upsert / update import — _"the Sequel Pro import"_ (Tier 2, high priority)

**Goal:** Pick a CSV → map columns → choose a match key (PK or unique column set) → update matched rows' selected fields with CSV values, optionally insert unmatched rows. Dry-run preview, single transactional commit.

**Why it's mostly frontend:** A CSV upsert is exactly a `TableChangeRequest`:
- `primary_key` = the user-chosen **match key** columns.
- For each CSV row that matches an existing row → `RowChange::Update { keys: <match cols>, assignments: <selected fields> }`.
- For each unmatched CSV row (insert mode) → `RowChange::Insert { values }`.
The existing `build_postgres_plan` + `commit_table_changes` then handle SQL generation, transaction, and validation.

**Frontend work (new import wizard modal, `apps/desktop/src/components/modals/ImportDataModal.tsx`):**
1. File picker (`tauri-plugin-dialog`) + CSV parse (header detection, delimiter sniff, quote handling). Stream/iterate for large files.
2. **Column mapping** UI: CSV column → table column, with type hints from introspected `ColumnMeta`. Auto-map by name.
3. **Match-key selector**: choose one or more columns to match on; default to the table PK (already in schema metadata). Validate the key columns are present in the CSV mapping.
4. **Mode toggle**: Update-only · Insert-only · Upsert (update matched + insert unmatched). Update-only with no match = skip; show counts.
5. **Field selection**: which mapped columns to actually write on update (Sequel Pro's "update all selected fields"). Match-key columns excluded from the SET list.
6. **Dry-run preview**: show "N rows match → update · M rows new → insert · K unmatched → skipped", reusing `preview_table_changes` IPC (already exists per the generated command list) to render the generated SQL before commit.
7. Commit via the existing `commit_table_changes` path.

**Backend work (small):**
- To detect which CSV rows match existing rows, either (a) let the DB decide via `INSERT ... ON CONFLICT (match_key) DO UPDATE` for the upsert case (Postgres-native, fastest, atomic), or (b) pre-query existing keys. Recommend adding an **`RowChange::Upsert`** variant (or an `on_conflict` flag on the plan) to `cellar-diff` so upsert compiles to `INSERT ... ON CONFLICT DO UPDATE SET ...` in one statement per row/batch. This keeps it transactional and avoids a read-then-write race.
- Batch inserts (multi-row `VALUES`) for throughput on large CSVs rather than one statement per row.

**Acceptance:** Import a 10k-row CSV, match on `id`, update `name`+`status` only, insert new ids; preview shows accurate match/insert/skip counts; commit is one transaction; re-running is idempotent; match-key columns are never overwritten.

**Stretch:** remember mappings per table; support TSV/Excel paste into the same wizard (folds in Tier 2 #4).

---

## W2. Faster dumps & restores (Tier 3 / new "Data movement" area, medium)

**Scope guard:** SPEC §2 lists "backup orchestration" as a non-goal. Frame this as **developer-convenience dump/restore**, not cluster/backup management: export a table / schema / database to a `.sql` or archive, and restore from one. Keep it desktop-first and local.

**Backend work:**
- New driver capability (free functions like `browse_table`, or a small trait extension): `dump(conn, scope, options) -> stream` and `restore(conn, source) -> progress`.
  - **Postgres:** two paths — (a) shell out to `pg_dump`/`pg_restore` when present on PATH (fastest, most faithful, supports custom/directory formats), surfacing version-mismatch errors clearly; (b) a native fallback using `COPY ... TO STDOUT` / `COPY ... FROM STDIN` for data-only export when the binaries aren't available. Schema-only dumps can reuse introspection + a DDL generator.
  - Scope selector: whole DB · selected schemas · selected tables · data-only / schema-only / both.
- **Streaming**: pipe `pg_dump` stdout (or COPY stream) straight to the target file — never materialize the whole dump in memory. Emit progress events over IPC (bytes written / rows copied).
- Restore: stream the file into `psql`/`pg_restore` or `COPY FROM`, with progress + a clear error surface on failure.

**Frontend work:**
- Entry points in the connection / database / table context menus ("Dump…", "Restore…").
- A dump/restore modal: scope, format, options, destination path; live progress bar fed by IPC events; cancel button (ties into the cancellation work from Top 5 #2).

**Acceptance:** Dump a mult-GB table to a file with a live progress bar and flat memory usage; restore it into another connection; cancel mid-dump cleanly; clear error if `pg_dump` is missing or version-mismatched, with the native COPY fallback offered for data-only.

---

## W3. Quick filter + advanced filter coexistence (Tier 2, medium)

**Goal:** A fast quick-filter (type an ID / free text) that lives **alongside** structured advanced filters without either clobbering the other — both visible, independently clearable, with a clear active-state indicator.

**Why it's mostly UI/state:** `TableBrowseRequest.filters: Vec<TableFilterClause>` (`crates/cellar-core/src/query.rs:200`) already supports multiple clauses combined server-side, and the Postgres/SQL Server `table_browse` paths already AND them together. So the backend can already express "quick + advanced" — this is a front-end composition problem.

**Frontend work (`@cellar/data-grid` filter bar + `BottomPanel.tsx` / table tab state):**
1. Model two distinct filter layers in tab state: `quickFilter` (string) and `advancedFilters` (`TableFilterClause[]`). Keep them separate so clearing one doesn't touch the other.
2. **Compile quick filter → clause(s)**: by default, a free-text quick filter becomes an OR across text-ish columns (`ILIKE`/Contains); if it's numeric and the table has an obvious id/PK, offer "match id = N". Send the union as additional `TableFilterClause`s appended to the advanced ones in the `TableBrowseRequest`.
   - Note: `TableFilterClause`s are currently AND-combined in the drivers. To support an OR'd quick filter across columns, either add a lightweight `group`/`logic` notion to the filter model, or (simpler v1) scope the quick filter to a single user-chosen column so it's one clause. Pick the simpler v1, leave OR-across-columns as a follow-up.
3. **Both visible**: quick-filter input pinned in the toolbar; advanced filter chips below. Each shows its own clear (×). A combined "N filters active" indicator.
4. Debounce quick-filter input; re-issue `browse_table` with the merged filter set; preserve sort + pagination (ties into Top 5 #1).

**Acceptance:** Set an advanced filter (`status = 'active'`), then type an id in quick-filter; both apply together; clearing the quick filter leaves the advanced filter intact and vice-versa; the active-filter indicator reflects both; results page/sort correctly with both applied.

---

## Where these slot in the build order

- **W3** is the cheapest (mostly grid state) and pairs well with the **Top 5 #1 pagination** work since both touch the browse path and grid — do them together.
- **W1 (CSV upsert)** is high user-value and reuses `cellar-diff`; schedule it right after the grid/commit groundwork is solid. The one backend addition is an `Upsert`/`ON CONFLICT` variant in `cellar-diff`.
- **W2 (dumps/restores)** is the largest and most independent; treat it as its own "Data movement" workstream, sequenced after the core grid/query work lands.
