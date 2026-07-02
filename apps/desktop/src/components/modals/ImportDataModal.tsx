import { useMemo, useRef, useState } from "react";
import { commands, unwrap, type Table } from "@cellar/ipc";

import {
  autoMap,
  buildImportRequest,
  importCounts,
  parseCsv,
  validateImport,
  type ImportColumn,
  type ImportConfig,
  type ImportMode,
  type ParsedCsv,
} from "../../lib/csvImport";
import { useConnections } from "../../state/connections";
import { useStatus } from "../../state/status";
import { useTabs, type TableTab } from "../../state/tabs";
import { renderTokens, tokenizeSql, tokensToLines } from "../../lib/sqlTokens";
import { Icon } from "../icons";
import { Modal } from "./Modal";
import { ED_RUN_PRIMARY, ED_RUN_SUBTLE, Section } from "./settingsPrimitives";

const MODES: { value: ImportMode; label: string; hint: string }[] = [
  { value: "update", label: "Update only", hint: "Change matched rows; ignore rows with no match." },
  { value: "insert", label: "Insert only", hint: "Add new rows; skip rows that already exist." },
  { value: "upsert", label: "Upsert", hint: "Update matched rows and insert the rest." },
];

type Step = "source" | "configure" | "preview";

export function ImportDataModal({ onClose }: { onClose: () => void }) {
  const byId = useConnections((s) => s.byId);
  const connections = useConnections((s) => s.connections);
  const refreshTable = useTabs((s) => s.refreshTable);
  const activeId = useTabs((s) => s.activeId);
  const tabs = useTabs((s) => s.tabs);
  const active = tabs.find((t) => t.id === activeId) ?? null;
  const tab = active?.kind === "table" ? active : null;

  const table = useMemo(() => (tab ? findTableMeta(byId, tab) : null), [byId, tab]);
  const conn = tab
    ? connections.find((c) => c.id === tab.connectionId) ?? null
    : null;
  const tableColumns: ImportColumn[] = useMemo(
    () =>
      (table?.columns ?? []).map((c) => ({
        name: c.name,
        data_type: c.data_type,
        nullable: c.nullable,
        is_primary_key: c.is_primary_key,
        has_default: c.default !== null,
      })),
    [table],
  );

  const [step, setStep] = useState<Step>("source");
  const [fileName, setFileName] = useState<string | null>(null);
  const [csv, setCsv] = useState<ParsedCsv | null>(null);
  const [parseError, setParseError] = useState<string | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);

  const [mapping, setMapping] = useState<Record<string, number>>({});
  const [matchKeys, setMatchKeys] = useState<string[]>([]);
  const [mode, setMode] = useState<ImportMode>("upsert");
  const [updateFields, setUpdateFields] = useState<string[]>([]);

  const [previewSql, setPreviewSql] = useState<string | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [committing, setCommitting] = useState(false);
  const [commitError, setCommitError] = useState<string | null>(null);
  const [result, setResult] = useState<{ rows: number; ms: number } | null>(null);

  if (!tab) {
    return (
      <Modal onClose={onClose} width={420}>
        <div className="flex items-center gap-2 px-4 py-5 text-sm text-fg-1">
          <Icon.warn size={13} stroke="var(--fg-3)" />
          <span>Open a table before importing data.</span>
        </div>
      </Modal>
    );
  }

  const cfg: ImportConfig = {
    database: tab.database,
    schema: tab.schema,
    table: tab.table,
    tableColumns,
    mapping,
    matchKeys,
    mode,
    updateFields,
  };
  const blockers = csv ? validateImport(csv, cfg) : [];
  const counts = csv ? importCounts(csv, cfg) : null;

  const onFile = async (file: File | undefined) => {
    if (!file) return;
    setFileName(file.name);
    setParseError(null);
    try {
      const parsed = parseCsv(await file.text());
      if (parsed.headers.length === 0 || parsed.rows.length === 0) {
        setParseError("That CSV has no header row or no data rows.");
        setCsv(null);
        return;
      }
      // Sensible defaults: auto-map by name, match on the PK, update the rest.
      const auto = autoMap(parsed.headers, tableColumns);
      const pk = table?.primary_key.filter((k) => auto[k] !== undefined) ?? [];
      setMapping(auto);
      setMatchKeys(pk);
      setUpdateFields(
        tableColumns
          .map((c) => c.name)
          .filter((name) => auto[name] !== undefined && !pk.includes(name)),
      );
      setCsv(parsed);
      setStep("configure");
    } catch {
      setParseError("Could not read that file.");
      setCsv(null);
    }
  };

  const remap = (col: string, csvIndex: number | null) =>
    setMapping((m) => {
      const next = { ...m };
      if (csvIndex === null) delete next[col];
      else next[col] = csvIndex;
      return next;
    });

  const toggleKey = (col: string) =>
    setMatchKeys((keys) => {
      if (keys.includes(col)) return keys.filter((k) => k !== col);
      // a match key is never also in the update set
      setUpdateFields((f) => f.filter((x) => x !== col));
      return [...keys, col];
    });

  const toggleField = (col: string) =>
    setUpdateFields((f) =>
      f.includes(col) ? f.filter((x) => x !== col) : [...f, col],
    );

  const goPreview = async () => {
    if (!csv || blockers.length > 0) return;
    setCommitError(null);
    setPreviewError(null);
    setStep("preview");
    // Preview a representative sample so we never build a 10k-line SQL string.
    const sample: ParsedCsv = { headers: csv.headers, rows: csv.rows.slice(0, 25) };
    try {
      const preview = await unwrap(
        commands.previewTableChanges(buildImportRequest(sample, cfg)),
      );
      setPreviewSql(preview.sql);
    } catch (err) {
      setPreviewError(err instanceof Error ? err.message : String(err));
    }
  };

  const commit = async () => {
    if (!csv) return;
    setCommitting(true);
    setCommitError(null);
    try {
      const res = await unwrap(
        commands.commitTableImport(
          tab.connectionId,
          buildImportRequest(csv, cfg),
          tab.id,
        ),
      );
      useStatus.getState().setLastQuery({
        connectionId: tab.connectionId,
        tabId: tab.id,
        rowCount: res.rows_affected,
        truncated: false,
        durationMs: res.duration_ms,
      });
      refreshTable(tab.id);
      setResult({ rows: res.rows_affected, ms: res.duration_ms });
    } catch (err) {
      setCommitError(err instanceof Error ? err.message : String(err));
    } finally {
      setCommitting(false);
    }
  };

  const sqlLines = previewSql ? tokensToLines(tokenizeSql(previewSql)) : [];

  return (
    <Modal onClose={onClose} width={760} height={result ? undefined : 640}>
      <div className="flex h-[38px] shrink-0 items-center justify-between border-b border-border-default pl-3.5 pr-2">
        <div className="flex min-w-0 items-center gap-2">
          <span className="inline-flex text-accent">
            <Icon.upload size={14} />
          </span>
          <span className="whitespace-nowrap text-sm font-semibold text-fg-0">
            Import data
          </span>
          <span className="ml-1 min-w-0 overflow-hidden text-ellipsis whitespace-nowrap border-l border-border-divider pl-1.5 font-mono text-sm text-fg-2">
            {tab.schema}.{tab.table}{" "}
            <span style={{ color: "var(--fg-3)" }}>·</span>{" "}
            {conn?.name ?? "no connection"}
            {conn?.env_tag ? ` (${conn.env_tag})` : ""}
          </span>
        </div>
        <button type="button" className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

      {result ? (
        <ResultView result={result} />
      ) : !table ? (
        <div className="flex-1 px-4 py-4 text-sm text-warn">
          Schema metadata for this table isn't loaded yet — open the table once,
          then try again.
        </div>
      ) : step === "source" ? (
        <SourceView
          fileName={fileName}
          error={parseError}
          fileRef={fileRef}
          onFile={onFile}
        />
      ) : step === "configure" ? (
        <ConfigureView
          csv={csv!}
          tableColumns={tableColumns}
          mapping={mapping}
          matchKeys={matchKeys}
          updateFields={updateFields}
          mode={mode}
          setMode={setMode}
          remap={remap}
          toggleKey={toggleKey}
          toggleField={toggleField}
          blockers={blockers}
        />
      ) : (
        <PreviewView
          counts={counts!}
          mode={mode}
          sqlLines={sqlLines}
          previewError={previewError}
          totalRows={csv?.rows.length ?? 0}
        />
      )}

      {result ? (
        <div className="flex h-11 shrink-0 items-center justify-end border-t border-border-default bg-bg-2 px-3">
          <button className={ED_RUN_PRIMARY} onClick={onClose}>
            <Icon.check size={11} />
            <span>Done</span>
          </button>
        </div>
      ) : null}

      {!result && table && (
        <div className="flex h-11 shrink-0 items-center justify-between gap-3 border-t border-border-default bg-bg-2 px-3">
          <span className="min-w-0 truncate text-[11.5px] text-fg-2">
            {commitError ? (
              <span className="text-delete">{commitError}</span>
            ) : step === "source" ? (
              "Pick a CSV with a header row."
            ) : counts ? (
              `${counts.total} rows · ${counts.toWrite} to write${counts.skipped ? ` · ${counts.skipped} skipped (no match key)` : ""}`
            ) : (
              ""
            )}
          </span>
          <div className="flex shrink-0 items-center gap-2">
            {step !== "source" && (
              <button
                className={ED_RUN_SUBTLE}
                onClick={() => setStep(step === "preview" ? "configure" : "source")}
              >
                <Icon.chevronLeft size={11} />
                <span>Back</span>
              </button>
            )}
            {step === "configure" && (
              <button
                className={ED_RUN_PRIMARY + " disabled:cursor-not-allowed disabled:opacity-40"}
                onClick={() => void goPreview()}
                disabled={blockers.length > 0}
                title={blockers[0]}
              >
                <span>Preview</span>
                <Icon.chevronRight size={11} />
              </button>
            )}
            {step === "preview" && (
              <button
                className={ED_RUN_PRIMARY + " disabled:cursor-not-allowed disabled:opacity-40"}
                onClick={() => void commit()}
                disabled={committing || !!previewError}
              >
                <Icon.check size={11} />
                <span>{committing ? "Importing…" : "Commit import"}</span>
              </button>
            )}
          </div>
        </div>
      )}
    </Modal>
  );
}

function findTableMeta(
  byId: ReturnType<typeof useConnections.getState>["byId"],
  tab: TableTab,
): Table | null {
  return (
    byId[tab.connectionId]?.databases
      .find((d) => d.name === tab.database)
      ?.schemas.find((s) => s.name === tab.schema)
      ?.tables.find((t) => t.name === tab.table) ?? null
  );
}

// ---------------------------------------------------------------------------
// Step 1 — source
// ---------------------------------------------------------------------------

function SourceView({
  fileName,
  error,
  fileRef,
  onFile,
}: {
  fileName: string | null;
  error: string | null;
  fileRef: React.RefObject<HTMLInputElement>;
  onFile: (file: File | undefined) => void;
}) {
  return (
    <div className="flex-1 overflow-y-auto px-4 py-3.5">
      <p className="m-0 mb-3 max-w-[60ch] text-sm text-fg-2 text-pretty">
        Load a CSV file to update or insert rows in this table. Its first row is
        treated as the header. You'll map columns, choose a match key, and review
        the generated SQL before anything is committed.
      </p>
      <input
        ref={fileRef}
        type="file"
        accept=".csv,.tsv,text/csv,text/tab-separated-values"
        className="hidden"
        onChange={(e) => onFile(e.target.files?.[0])}
      />
      <button
        type="button"
        onClick={() => fileRef.current?.click()}
        className="flex w-full items-center justify-center gap-2 rounded-[6px] border border-dashed border-border-strong bg-bg-2 px-3 py-6 text-sm text-fg-1 hover:border-accent-line hover:bg-accent-soft"
      >
        <Icon.fileText size={14} stroke="var(--fg-2)" />
        <span>{fileName ? `Loaded ${fileName} — choose another` : "Choose a .csv file"}</span>
      </button>
      {error && (
        <div className="mt-2.5 flex items-start gap-1.5 rounded-[4px] border border-[color-mix(in_oklab,var(--delete)_30%,var(--border-default))] bg-delete-bg px-3 py-2 text-sm text-delete">
          <Icon.warn size={12} />
          <span>{error}</span>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 2 — configure
// ---------------------------------------------------------------------------

function ConfigureView({
  csv,
  tableColumns,
  mapping,
  matchKeys,
  updateFields,
  mode,
  setMode,
  remap,
  toggleKey,
  toggleField,
  blockers,
}: {
  csv: ParsedCsv;
  tableColumns: ImportColumn[];
  mapping: Record<string, number>;
  matchKeys: string[];
  updateFields: string[];
  mode: ImportMode;
  setMode: (m: ImportMode) => void;
  remap: (col: string, csvIndex: number | null) => void;
  toggleKey: (col: string) => void;
  toggleField: (col: string) => void;
  blockers: string[];
}) {
  return (
    <div className="flex-1 overflow-y-auto pb-3">
      <Section title="Mode">
        <div className="flex flex-col gap-1">
          {MODES.map((m) => (
            <button
              key={m.value}
              type="button"
              onClick={() => setMode(m.value)}
              className={
                "flex items-center gap-2.5 rounded-[5px] border px-3 py-1.5 text-left " +
                (mode === m.value
                  ? "border-accent-line bg-accent-soft"
                  : "border-border-default bg-bg-2 hover:border-border-strong")
              }
            >
              <span
                className={
                  "inline-flex h-[13px] w-[13px] shrink-0 items-center justify-center rounded-full border " +
                  (mode === m.value
                    ? "border-accent bg-accent text-accent-fg"
                    : "border-border-strong bg-bg-inset")
                }
              >
                {mode === m.value && <Icon.check size={8} />}
              </span>
              <span>
                <span className="block text-sm font-medium text-fg-0">{m.label}</span>
                <span className="block text-[11.5px] text-fg-3">{m.hint}</span>
              </span>
            </button>
          ))}
        </div>
      </Section>

      <Section
        title="Columns"
        sub="Map each table column to a CSV column. Pick the match key (PK is unique-constraint backed for upsert/insert) and which fields to write."
      >
        <div className="grid grid-cols-[1fr_140px_auto] items-center gap-x-2 px-1 pb-1 text-[10.5px] font-semibold uppercase tracking-[0.05em] text-fg-3">
          <span>Table column</span>
          <span>From CSV</span>
          <span className="text-right">Role</span>
        </div>
        <div className="flex flex-col gap-px">
          {tableColumns.map((col) => {
            const isKey = matchKeys.includes(col.name);
            const mapped = mapping[col.name] !== undefined;
            return (
              <div
                key={col.name}
                className="grid grid-cols-[1fr_140px_auto] items-center gap-x-2 rounded-[4px] border border-border-default bg-bg-2 px-2 py-1"
              >
                <span className="flex min-w-0 items-center gap-1.5">
                  <span className="truncate font-mono text-sm text-fg-0">
                    {col.name}
                  </span>
                  {col.is_primary_key && <Tag tone="accent">PK</Tag>}
                  {!col.nullable && <Tag tone="warn">NOT NULL</Tag>}
                  <span className="truncate font-mono text-[11px] text-fg-3">
                    {col.data_type}
                  </span>
                </span>
                <select
                  value={mapping[col.name] ?? ""}
                  onChange={(e) =>
                    remap(col.name, e.target.value === "" ? null : Number(e.target.value))
                  }
                  className="h-[24px] w-full rounded-[4px] border border-border-default bg-bg-inset px-1.5 text-sm text-fg-0 outline-none focus:border-accent-line"
                >
                  <option value="">(skip)</option>
                  {csv.headers.map((h, i) => (
                    <option key={i} value={i}>
                      {h || `col ${i + 1}`}
                    </option>
                  ))}
                </select>
                <span className="flex shrink-0 items-center justify-end gap-1">
                  <RoleToggle
                    label="key"
                    active={isKey}
                    disabled={!mapped}
                    onClick={() => toggleKey(col.name)}
                    title="Match on this column"
                  />
                  {mode !== "insert" && (
                    <RoleToggle
                      label="update"
                      active={updateFields.includes(col.name) && !isKey}
                      disabled={!mapped || isKey}
                      onClick={() => toggleField(col.name)}
                      title={isKey ? "Match keys are never overwritten" : "Write this field"}
                    />
                  )}
                </span>
              </div>
            );
          })}
        </div>
      </Section>

      {blockers.length > 0 && (
        <div className="mx-4 mt-1 flex flex-col gap-0.5 rounded-[4px] border border-[color-mix(in_oklab,var(--warn)_30%,var(--border-default))] bg-warn-bg px-3 py-2 text-sm text-warn">
          {blockers.map((b) => (
            <div key={b} className="flex items-start gap-1.5">
              <Icon.warn size={11} />
              <span>{b}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 3 — preview
// ---------------------------------------------------------------------------

function PreviewView({
  counts,
  mode,
  sqlLines,
  previewError,
  totalRows,
}: {
  counts: { total: number; toWrite: number; skipped: number };
  mode: ImportMode;
  sqlLines: ReturnType<typeof tokensToLines>;
  previewError: string | null;
  totalRows: number;
}) {
  const verb =
    mode === "update" ? "update where matched" : mode === "insert" ? "insert where new" : "insert or update";
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex shrink-0 items-center gap-4 border-b border-border-default bg-bg-2 px-4 py-2.5">
        <Stat n={counts.toWrite} label={`rows → ${verb}`} color="var(--accent)" />
        {counts.skipped > 0 && (
          <Stat n={counts.skipped} label="skipped (no match key)" color="var(--fg-3)" />
        )}
        <div className="flex-1" />
        <span className="inline-flex items-center gap-1.5 whitespace-nowrap rounded-[4px] bg-bg-inset px-2 py-1 font-mono text-[11.5px] text-fg-2">
          <Icon.bracket size={10} />
          <span>BEGIN … COMMIT · atomic</span>
        </span>
      </div>
      <div className="flex h-[24px] shrink-0 items-center justify-between border-b border-border-divider bg-bg-1 px-3 text-[11px] font-semibold uppercase tracking-[0.05em] text-fg-3">
        <span>
          Generated SQL{totalRows > 25 ? " (first 25 rows shown)" : ""}
        </span>
      </div>
      <div className="flex-1 overflow-auto bg-bg-inset py-2 font-mono text-sm leading-[1.55]">
        {previewError ? (
          <div className="px-3 text-sm text-delete">{previewError}</div>
        ) : sqlLines.length === 0 ? (
          <div className="px-3 text-sm text-fg-3">Generating preview…</div>
        ) : (
          sqlLines.map((toks, i) => (
            <div key={i} className="flex px-3">
              <span className="inline-flex w-7 shrink-0 select-none items-center justify-end pr-2.5 font-mono text-[11px] text-fg-3">
                {i + 1}
              </span>
              <span className="whitespace-pre font-mono">{renderTokens(toks)}</span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function ResultView({ result }: { result: { rows: number; ms: number } }) {
  return (
    <div className="flex-1 px-4 py-5">
      <div className="mb-2 flex items-center gap-2">
        <span className="inline-flex h-6 w-6 items-center justify-center rounded-full bg-accent-soft text-accent">
          <Icon.check size={13} />
        </span>
        <span className="text-sm font-semibold text-fg-0">Import committed</span>
      </div>
      <p className="m-0 text-sm text-fg-2">
        {result.rows} row{result.rows === 1 ? "" : "s"} affected in {result.ms} ms.
        The table has been refreshed.
      </p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// bits
// ---------------------------------------------------------------------------

function Tag({ tone, children }: { tone: "accent" | "warn"; children: React.ReactNode }) {
  return (
    <span
      className={
        "shrink-0 rounded-[3px] px-1 py-px font-mono text-[9.5px] uppercase tracking-[0.04em] " +
        (tone === "accent" ? "bg-accent-soft text-accent" : "bg-warn-bg text-warn")
      }
    >
      {children}
    </span>
  );
}

function RoleToggle({
  label,
  active,
  disabled,
  onClick,
  title,
}: {
  label: string;
  active: boolean;
  disabled?: boolean;
  onClick: () => void;
  title?: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      title={title}
      aria-pressed={active}
      className={
        "h-[20px] rounded-[3px] border px-1.5 text-[11px] " +
        (active
          ? "border-accent bg-accent text-accent-fg"
          : "border-border-default bg-bg-inset text-fg-2 hover:text-fg-0") +
        (disabled ? " cursor-not-allowed opacity-35" : "")
      }
    >
      {label}
    </button>
  );
}

function Stat({ n, label, color }: { n: number; label: string; color: string }) {
  return (
    <div className="flex items-center gap-1.5">
      <span className="font-mono text-[15px] font-semibold tabular-nums" style={{ color }}>
        {n}
      </span>
      <span className="text-sm text-fg-2">{label}</span>
    </div>
  );
}
