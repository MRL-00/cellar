import { useRef, useState } from "react";
import type { ConnectionConfig } from "@cellar/ipc";

import { useConnections } from "../../state/connections";
import { useTabs } from "../../state/tabs";
import { useSettings } from "../../lib/settings";
import {
  applyImportPlan,
  computeImportPlan,
  parseBundle,
  type ConnDecision,
  type ConnImportItem,
  type ImportPlan,
  type ImportResult,
  type LayoutDecision,
  type LayoutImportItem,
} from "../../lib/setupTransfer";
import { Icon } from "../icons";
import { ED_RUN_PRIMARY, ED_RUN_SUBTLE, Section } from "./settingsPrimitives";
import { Modal } from "./Modal";

export function ImportSetupModal({ onClose }: { onClose: () => void }) {
  const connections = useConnections((s) => s.connections);
  const saveConnection = useConnections((s) => s.saveConnection);
  const tableLayouts = useTabs((s) => s.tableLayouts);
  const importTableLayouts = useTabs((s) => s.importTableLayouts);
  const { importSettings } = useSettings();

  const [raw, setRaw] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [fileName, setFileName] = useState<string | null>(null);
  const [plan, setPlan] = useState<ImportPlan | null>(null);
  const [applying, setApplying] = useState(false);
  const [result, setResult] = useState<ImportResult | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);

  const tryParse = (text: string) => {
    const parsed = parseBundle(text);
    if (!parsed.ok) {
      setError(parsed.error);
      setPlan(null);
      return;
    }
    setError(null);
    setPlan(
      computeImportPlan(parsed.bundle, {
        connections,
        tableLayouts,
      }),
    );
  };

  const onFile = async (file: File | undefined) => {
    if (!file) return;
    setFileName(file.name);
    try {
      const text = await file.text();
      setRaw(text);
      tryParse(text);
    } catch {
      setError("Could not read that file.");
    }
  };

  const setConnDecision = (idx: number, decision: ConnDecision) =>
    setPlan((p) =>
      p
        ? {
            ...p,
            connections: p.connections.map((c, i) =>
              i === idx ? { ...c, decision } : c,
            ),
          }
        : p,
    );

  const setLayoutDecision = (idx: number, decision: LayoutDecision) =>
    setPlan((p) =>
      p
        ? {
            ...p,
            layouts: p.layouts.map((l, i) =>
              i === idx ? { ...l, decision } : l,
            ),
          }
        : p,
    );

  const bulkConn = (fn: (c: ConnImportItem) => ConnDecision) =>
    setPlan((p) =>
      p
        ? { ...p, connections: p.connections.map((c) => ({ ...c, decision: fn(c) })) }
        : p,
    );

  const bulkLayout = (fn: (l: LayoutImportItem) => LayoutDecision) =>
    setPlan((p) =>
      p ? { ...p, layouts: p.layouts.map((l) => ({ ...l, decision: fn(l) })) } : p,
    );

  const setSettingsApply = (apply: boolean) =>
    setPlan((p) =>
      p && p.settings ? { ...p, settings: { ...p.settings, apply } } : p,
    );

  const hasActions = Boolean(
    plan &&
      (plan.connections.some((c) => c.decision !== "skip") ||
        plan.layouts.some((l) => l.decision !== "skip") ||
        plan.settings?.apply),
  );

  const onApply = async () => {
    if (!plan) return;
    setApplying(true);
    try {
      const res = await applyImportPlan(plan, {
        existingConnectionIds: connections.map((c) => c.id),
        saveConnection,
        importSettings,
        importTableLayouts,
      });
      setResult(res);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setApplying(false);
    }
  };

  return (
    <Modal onClose={onClose} width={620} height={result ? undefined : 600}>
      <div className="flex h-[38px] shrink-0 items-center justify-between border-b border-border-default pl-3.5 pr-2">
        <div className="flex items-center gap-2">
          <span className="inline-flex text-accent">
            <Icon.upload size={14} />
          </span>
          <span className="whitespace-nowrap text-[12.5px] font-semibold text-fg-0">
            Import setup
          </span>
        </div>
        <button className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

      {result ? (
        <ResultView result={result} />
      ) : plan ? (
        <ReviewView
          plan={plan}
          setConnDecision={setConnDecision}
          setLayoutDecision={setLayoutDecision}
          bulkConn={bulkConn}
          bulkLayout={bulkLayout}
          setSettingsApply={setSettingsApply}
        />
      ) : (
        <SourceView
          raw={raw}
          setRaw={setRaw}
          error={error}
          fileName={fileName}
          fileRef={fileRef}
          onFile={onFile}
          onReview={() => tryParse(raw)}
        />
      )}

      <div className="flex h-11 shrink-0 items-center justify-between gap-3 border-t border-border-default bg-bg-2 px-3">
        {result ? (
          <>
            <span className="text-[11px] text-fg-2">Import complete.</span>
            <button className={ED_RUN_PRIMARY} onClick={onClose}>
              <Icon.check size={11} />
              <span>Done</span>
            </button>
          </>
        ) : plan ? (
          <>
            <button
              className={ED_RUN_SUBTLE}
              onClick={() => {
                setPlan(null);
                setError(null);
              }}
            >
              <Icon.chevronLeft size={11} />
              <span>Back</span>
            </button>
            <button
              className={ED_RUN_PRIMARY + " disabled:cursor-not-allowed disabled:opacity-40"}
              onClick={() => void onApply()}
              disabled={!hasActions || applying}
            >
              <Icon.check size={11} />
              <span>{applying ? "Applying…" : "Apply import"}</span>
            </button>
          </>
        ) : (
          <>
            <span className="font-mono text-[10.5px] text-fg-3">
              {fileName ?? "no file selected"}
            </span>
            <button
              className={ED_RUN_PRIMARY + " disabled:cursor-not-allowed disabled:opacity-40"}
              onClick={() => tryParse(raw)}
              disabled={!raw.trim()}
            >
              <span>Review</span>
              <Icon.chevronRight size={11} />
            </button>
          </>
        )}
      </div>
    </Modal>
  );
}

// ---------------------------------------------------------------------------
// Step 1 — source
// ---------------------------------------------------------------------------

function SourceView({
  raw,
  setRaw,
  error,
  fileName,
  fileRef,
  onFile,
  onReview,
}: {
  raw: string;
  setRaw: (v: string) => void;
  error: string | null;
  fileName: string | null;
  fileRef: React.RefObject<HTMLInputElement>;
  onFile: (file: File | undefined) => void;
  onReview: () => void;
}) {
  return (
    <div className="flex-1 overflow-y-auto px-4 py-3.5">
      <p className="m-0 mb-3 max-w-[56ch] text-[11.5px] text-fg-2 text-pretty">
        Load a Cellar setup file someone shared with you. You'll review each item
        before anything changes.
      </p>

      <input
        ref={fileRef}
        type="file"
        accept=".json,application/json"
        className="hidden"
        onChange={(e) => onFile(e.target.files?.[0])}
      />
      <button
        type="button"
        onClick={() => fileRef.current?.click()}
        className="flex w-full items-center justify-center gap-2 rounded-[6px] border border-dashed border-border-strong bg-bg-2 px-3 py-5 text-[12px] text-fg-1 hover:border-accent-line hover:bg-accent-soft"
      >
        <Icon.fileText size={14} stroke="var(--fg-2)" />
        <span>{fileName ? `Loaded ${fileName} — choose another` : "Choose a .json file"}</span>
      </button>

      <div className="my-3 flex items-center gap-2 text-[10.5px] text-fg-3">
        <span className="h-px flex-1 bg-border-divider" />
        <span>or paste JSON</span>
        <span className="h-px flex-1 bg-border-divider" />
      </div>

      <textarea
        value={raw}
        onChange={(e) => setRaw(e.target.value)}
        onBlur={() => raw.trim() && onReview()}
        spellCheck={false}
        placeholder={'{\n  "format": "cellar.setup",\n  ...\n}'}
        className="h-[180px] w-full resize-none rounded-[5px] border border-border-default bg-bg-inset px-2.5 py-2 font-mono text-[11px] text-fg-0 outline-none focus:border-accent-line"
      />

      {error && (
        <div className="mt-2.5 flex items-start gap-1.5 rounded-[4px] border border-[color-mix(in_oklab,var(--delete)_30%,var(--border-default))] bg-delete-bg px-3 py-2 text-[11px] text-delete">
          <Icon.warn size={12} />
          <span>{error}</span>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 2 — review
// ---------------------------------------------------------------------------

function ReviewView({
  plan,
  setConnDecision,
  setLayoutDecision,
  bulkConn,
  bulkLayout,
  setSettingsApply,
}: {
  plan: ImportPlan;
  setConnDecision: (idx: number, d: ConnDecision) => void;
  setLayoutDecision: (idx: number, d: LayoutDecision) => void;
  bulkConn: (fn: (c: ConnImportItem) => ConnDecision) => void;
  bulkLayout: (fn: (l: LayoutImportItem) => LayoutDecision) => void;
  setSettingsApply: (apply: boolean) => void;
}) {
  return (
    <div className="flex-1 overflow-y-auto pb-4">
      {plan.settings && (
        <Section title="Appearance & settings">
          <button
            type="button"
            onClick={() => setSettingsApply(!plan.settings!.apply)}
            className={
              "flex items-center gap-2.5 rounded-[5px] border px-3 py-2 text-left " +
              (plan.settings.apply
                ? "border-accent-line bg-accent-soft"
                : "border-border-default bg-bg-2 hover:border-border-strong")
            }
          >
            <span
              className={
                "inline-flex h-[15px] w-[15px] shrink-0 items-center justify-center rounded-[4px] border " +
                (plan.settings.apply
                  ? "border-accent bg-accent text-accent-fg"
                  : "border-border-strong bg-bg-inset")
              }
            >
              {plan.settings.apply && <Icon.check size={10} />}
            </span>
            <span className="min-w-0">
              <span className="block text-[12px] font-medium text-fg-0">
                Apply imported appearance & settings
              </span>
              <span className="flex items-center gap-1.5 text-[10.5px] text-fg-3">
                <span
                  className="inline-block h-2.5 w-2.5 rounded-full border border-white/15"
                  style={{ background: plan.settings.settings.accent }}
                />
                {plan.settings.settings.theme} · {plan.settings.settings.density} ·{" "}
                {plan.settings.settings.interfaceFont} /{" "}
                {plan.settings.settings.monoFont}
              </span>
            </span>
          </button>
        </Section>
      )}

      {plan.connections.length > 0 && (
        <Section
          title={`Connections (${plan.connections.length})`}
          sub="Duplicates (same engine, host, database and user) are skipped by default so nothing imports twice."
        >
          <BulkBar>
            <BulkBtn onClick={() => bulkConn((c) => (c.duplicateOfId ? c.decision : "add"))}>
              Add all new
            </BulkBtn>
            <BulkBtn
              onClick={() => bulkConn((c) => (c.duplicateOfId ? "replace" : c.decision))}
            >
              Replace all duplicates
            </BulkBtn>
            <BulkBtn onClick={() => bulkConn(() => "skip")}>Skip all</BulkBtn>
          </BulkBar>
          <div className="flex flex-col gap-1">
            {plan.connections.map((item, idx) => (
              <ConnRow
                key={`${item.identity}-${idx}`}
                item={item}
                onChange={(d) => setConnDecision(idx, d)}
              />
            ))}
          </div>
        </Section>
      )}

      {plan.layouts.length > 0 && (
        <Section title={`Table grid layouts (${plan.layouts.length})`}>
          <BulkBar>
            <BulkBtn onClick={() => bulkLayout((l) => (l.exists ? l.decision : "add"))}>
              Add all new
            </BulkBtn>
            <BulkBtn onClick={() => bulkLayout((l) => (l.exists ? "replace" : l.decision))}>
              Replace all existing
            </BulkBtn>
            <BulkBtn onClick={() => bulkLayout(() => "skip")}>Skip all</BulkBtn>
          </BulkBar>
          <div className="flex flex-col gap-1">
            {plan.layouts.map((item, idx) => (
              <LayoutRow
                key={`${item.key}-${idx}`}
                item={item}
                onChange={(d) => setLayoutDecision(idx, d)}
              />
            ))}
          </div>
        </Section>
      )}
    </div>
  );
}

function ConnRow({
  item,
  onChange,
}: {
  item: ConnImportItem;
  onChange: (d: ConnDecision) => void;
}) {
  const dup = Boolean(item.duplicateOfId);
  const options: { value: ConnDecision; label: string }[] = dup
    ? [
        { value: "skip", label: "Skip" },
        { value: "replace", label: "Replace" },
        { value: "copy", label: "Add copy" },
      ]
    : [
        { value: "add", label: "Add" },
        { value: "skip", label: "Skip" },
      ];
  return (
    <div className="flex items-center gap-2.5 rounded-[4px] border border-border-default bg-bg-2 px-2.5 py-1.5">
      <span className="min-w-0 flex-1">
        <span className="flex items-center gap-1.5">
          <span className="truncate text-[11.5px] font-medium text-fg-0">
            {item.incoming.name}
          </span>
          <StatusBadge
            tone={dup ? "warn" : "ok"}
            label={dup ? `dup of ${item.duplicateOfName}` : "new"}
          />
        </span>
        <span className="block truncate font-mono text-[10px] text-fg-3">
          {connHint(item.incoming)}
        </span>
      </span>
      <MiniSegment value={item.decision} options={options} onChange={onChange} />
    </div>
  );
}

function LayoutRow({
  item,
  onChange,
}: {
  item: LayoutImportItem;
  onChange: (d: LayoutDecision) => void;
}) {
  const options: { value: LayoutDecision; label: string }[] = item.exists
    ? [
        { value: "skip", label: "Skip" },
        { value: "replace", label: "Replace" },
      ]
    : [
        { value: "add", label: "Add" },
        { value: "skip", label: "Skip" },
      ];
  return (
    <div className="flex items-center gap-2.5 rounded-[4px] border border-border-default bg-bg-2 px-2.5 py-1.5">
      <span className="min-w-0 flex-1">
        <span className="flex items-center gap-1.5">
          <span className="truncate font-mono text-[11px] text-fg-0">
            {item.tablePath}
          </span>
          <StatusBadge
            tone={item.exists ? "warn" : "ok"}
            label={item.exists ? "exists" : "new"}
          />
        </span>
        <span className="block truncate font-mono text-[10px] text-fg-3">
          {item.connectionId || "—"} · {item.layout.order.length} cols
        </span>
      </span>
      <MiniSegment value={item.decision} options={options} onChange={onChange} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

function ResultView({ result }: { result: ImportResult }) {
  const lines: string[] = [];
  if (result.connectionsAdded) lines.push(`${result.connectionsAdded} connection(s) added`);
  if (result.connectionsReplaced)
    lines.push(`${result.connectionsReplaced} connection(s) replaced`);
  if (result.connectionsSkipped)
    lines.push(`${result.connectionsSkipped} connection(s) skipped`);
  if (result.layoutsAdded) lines.push(`${result.layoutsAdded} layout(s) added`);
  if (result.layoutsReplaced) lines.push(`${result.layoutsReplaced} layout(s) replaced`);
  if (result.layoutsSkipped) lines.push(`${result.layoutsSkipped} layout(s) skipped`);
  if (result.settingsApplied) lines.push("appearance & settings applied");
  const importedConns = result.connectionsAdded + result.connectionsReplaced;

  return (
    <div className="flex-1 overflow-y-auto px-4 py-4">
      <div className="mb-3 flex items-center gap-2">
        <span className="inline-flex h-6 w-6 items-center justify-center rounded-full bg-accent-soft text-accent">
          <Icon.check size={13} />
        </span>
        <span className="text-[13px] font-semibold text-fg-0">Setup imported</span>
      </div>
      <ul className="m-0 flex list-none flex-col gap-1 p-0">
        {lines.length ? (
          lines.map((line) => (
            <li key={line} className="flex items-center gap-1.5 text-[11.5px] text-fg-1">
              <span className="h-1 w-1 rounded-full bg-fg-3" />
              {line}
            </li>
          ))
        ) : (
          <li className="text-[11.5px] text-fg-2">Nothing was changed.</li>
        )}
      </ul>
      {importedConns > 0 && (
        <div className="mt-3 flex items-start gap-1.5 rounded-[4px] border border-dashed border-border-default bg-bg-inset px-3 py-2 text-[11px] text-fg-2">
          <Icon.lock size={12} stroke="var(--fg-3)" />
          <span>
            Imported connections have no password yet — open each one and set its
            credentials before connecting.
          </span>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Small shared bits
// ---------------------------------------------------------------------------

function StatusBadge({ tone, label }: { tone: "ok" | "warn"; label: string }) {
  return (
    <span
      className={
        "shrink-0 rounded-[3px] px-1.5 py-px font-mono text-[9px] uppercase tracking-[0.04em] " +
        (tone === "warn"
          ? "bg-warn-bg text-warn"
          : "bg-insert-bg text-insert")
      }
    >
      {label}
    </span>
  );
}

function MiniSegment<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
}) {
  return (
    <div className="inline-flex shrink-0 gap-px rounded-[4px] border border-border-default bg-bg-inset p-[2px]">
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          onClick={() => onChange(o.value)}
          aria-pressed={value === o.value}
          className={
            "h-5 rounded-[3px] px-2 text-[10.5px] " +
            (value === o.value
              ? "bg-bg-3 font-medium text-fg-0"
              : "text-fg-2 hover:text-fg-0")
          }
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

function BulkBar({ children }: { children: React.ReactNode }) {
  return (
    <div className="mb-1 flex items-center gap-1.5">
      <span className="text-[10px] uppercase tracking-[0.06em] text-fg-3">Bulk</span>
      {children}
    </div>
  );
}

function BulkBtn({
  onClick,
  children,
}: {
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="rounded-[3px] border border-border-default bg-bg-2 px-2 py-0.5 text-[10.5px] text-fg-2 hover:border-border-strong hover:text-fg-0"
    >
      {children}
    </button>
  );
}

function connHint(c: ConnectionConfig): string {
  return `${c.engine} · ${c.host}:${c.port}/${c.database} · ${c.user || "—"}`;
}
