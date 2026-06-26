import { useEffect, useMemo, useState } from "react";
import {
  commands,
  unwrap,
  type SchemaSnapshotMeta,
  type SchemaSource,
} from "@cellar/ipc";

import { Icon } from "../icons";
import { Modal } from "./Modal";
import { useConnections } from "../../state/connections";
import { useSchemaCompare } from "../../state/schemaCompare";
import { useTabs } from "../../state/tabs";

export interface ComparePreset {
  connectionId: string;
  database: string;
  schema?: string;
}

/** Picker state for one side of the comparison. */
type Picker =
  | { mode: "live"; connectionId: string; database: string; schema: string }
  | { mode: "snapshot"; snapshotId: string; schema: string };

function emptyLive(preset?: ComparePreset): Picker {
  return {
    mode: "live",
    connectionId: preset?.connectionId ?? "",
    database: preset?.database ?? "",
    schema: preset?.schema ?? "",
  };
}

export function SchemaCompareDialog({
  onClose,
  preset,
}: {
  onClose: () => void;
  preset?: ComparePreset | null;
}) {
  const connections = useConnections((s) => s.connections);
  const byId = useConnections((s) => s.byId);
  const openSchemaCompare = useTabs((s) => s.openSchemaCompare);
  const start = useSchemaCompare((s) => s.start);

  const [source, setSource] = useState<Picker>(() => emptyLive(preset ?? undefined));
  const [target, setTarget] = useState<Picker>(() => emptyLive());
  const [snapshots, setSnapshots] = useState<SchemaSnapshotMeta[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void reloadSnapshots();
  }, []);

  async function reloadSnapshots() {
    try {
      setSnapshots(await unwrap(commands.listSchemaSnapshots()));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  const sourceRef = toSource(source);
  const targetRef = toSource(target);
  const canCompare = sourceRef !== null && targetRef !== null && !busy;

  function handleCompare() {
    if (!sourceRef || !targetRef) return;
    const liveAnchor =
      source.mode === "live"
        ? { connectionId: source.connectionId, database: source.database }
        : target.mode === "live"
          ? { connectionId: target.connectionId, database: target.database }
          : { connectionId: "", database: "" };
    const title = `${sourceRef.schema} ↔ ${targetRef.schema}`;
    const tabId = openSchemaCompare(title, liveAnchor.connectionId, liveAnchor.database);
    void start(tabId, { source: sourceRef, target: targetRef });
    onClose();
  }

  return (
    <Modal onClose={onClose} width={760}>
      <div className="flex h-[38px] shrink-0 items-center justify-between border-b border-border-default pl-3.5 pr-2">
        <div className="flex items-center gap-2">
          <Icon.diff size={14} stroke="var(--accent)" />
          <span className="text-[12.5px] font-semibold text-fg-0">
            Compare schemas
          </span>
        </div>
        <button type="button" className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

      <div className="flex flex-col gap-3 overflow-y-auto p-3.5">
        <div className="grid grid-cols-2 gap-3">
          <SidePicker
            heading="Source"
            subtitle="changed to match target"
            picker={source}
            onChange={setSource}
            connections={connections}
            byId={byId}
            snapshots={snapshots}
          />
          <SidePicker
            heading="Target"
            subtitle="the desired end state"
            picker={target}
            onChange={setTarget}
            connections={connections}
            byId={byId}
            snapshots={snapshots}
          />
        </div>

        <SnapshotManager
          connections={connections}
          byId={byId}
          snapshots={snapshots}
          onChanged={reloadSnapshots}
          onError={setError}
        />

        {error && <div className="text-[11px] text-warn">{error}</div>}
      </div>

      <div className="flex min-h-11 shrink-0 items-center justify-between gap-3 border-t border-border-default bg-bg-2 px-3 py-2">
        <span className="text-[10.5px] text-fg-3">
          The migration transforms the source into the target. Generated SQL is
          reviewed before it runs.
        </span>
        <div className="flex items-center gap-2">
          <button
            type="button"
            className="inline-flex h-[26px] items-center rounded-[4px] border border-border-default bg-transparent px-2.5 text-[11.5px] text-fg-1 hover:bg-bg-3"
            onClick={onClose}
          >
            Cancel
          </button>
          <button
            type="button"
            disabled={!canCompare}
            onClick={handleCompare}
            className="inline-flex h-[26px] items-center gap-1.5 rounded-[4px] border border-transparent bg-accent px-2.5 text-[11.5px] font-medium text-white hover:brightness-[1.07] disabled:cursor-not-allowed disabled:opacity-60"
          >
            <Icon.diff size={11} />
            Compare
          </button>
        </div>
      </div>
    </Modal>
  );
}

type ConnState = ReturnType<typeof useConnections.getState>["byId"];
type ConnList = ReturnType<typeof useConnections.getState>["connections"];

function SidePicker({
  heading,
  subtitle,
  picker,
  onChange,
  connections,
  byId,
  snapshots,
}: {
  heading: string;
  subtitle: string;
  picker: Picker;
  onChange: (p: Picker) => void;
  connections: ConnList;
  byId: ConnState;
  snapshots: SchemaSnapshotMeta[];
}) {
  const connectedConns = connections.filter(
    (c) => byId[c.id]?.status === "connected",
  );

  const databases =
    picker.mode === "live"
      ? byId[picker.connectionId]?.databases ?? []
      : [];
  const liveSchemas =
    picker.mode === "live"
      ? databases.find((d) => d.name === picker.database)?.schemas ?? []
      : [];
  const snapshotSchemas =
    picker.mode === "snapshot"
      ? snapshots.find((s) => s.id === picker.snapshotId)?.schemas ?? []
      : [];

  // When a live connection is chosen but its schema hasn't been introspected,
  // load it so the database/schema dropdowns populate.
  useEffect(() => {
    if (picker.mode !== "live" || !picker.connectionId) return;
    const state = byId[picker.connectionId];
    if (state?.status === "connected" && state.databases.length === 0 && !state.loadingSchema) {
      void useConnections.getState().refreshSchema(picker.connectionId);
    }
  }, [picker, byId]);

  return (
    <div className="flex flex-col gap-2 rounded-[6px] border border-border-default bg-bg-2 p-2.5">
      <div className="flex items-baseline justify-between">
        <span className="text-[11.5px] font-semibold text-fg-0">{heading}</span>
        <span className="text-[10px] text-fg-3">{subtitle}</span>
      </div>

      <div className="flex gap-1">
        <ModeTab
          active={picker.mode === "live"}
          label="Live"
          onClick={() => onChange(emptyLive())}
        />
        <ModeTab
          active={picker.mode === "snapshot"}
          label="Snapshot"
          onClick={() => onChange({ mode: "snapshot", snapshotId: "", schema: "" })}
        />
      </div>

      {picker.mode === "live" ? (
        <>
          <Field label="Connection">
            <Select
              value={picker.connectionId}
              onChange={(connectionId) =>
                onChange({ mode: "live", connectionId, database: "", schema: "" })
              }
              placeholder={connectedConns.length ? "Select…" : "No connected databases"}
              options={connectedConns.map((c) => ({ value: c.id, label: c.name }))}
            />
          </Field>
          <Field label="Database">
            <Select
              value={picker.database}
              onChange={(database) =>
                onChange({ ...picker, database, schema: "" })
              }
              placeholder="Select…"
              options={databases.map((d) => ({ value: d.name, label: d.name }))}
            />
          </Field>
          <Field label="Schema">
            <Select
              value={picker.schema}
              onChange={(schema) => onChange({ ...picker, schema })}
              placeholder="Select…"
              options={liveSchemas.map((s) => ({ value: s.name, label: s.name }))}
            />
          </Field>
        </>
      ) : (
        <>
          <Field label="Snapshot">
            <Select
              value={picker.snapshotId}
              onChange={(snapshotId) =>
                onChange({ mode: "snapshot", snapshotId, schema: "" })
              }
              placeholder={snapshots.length ? "Select…" : "No snapshots saved"}
              options={snapshots.map((s) => ({
                value: s.id,
                label: `${s.label} (${formatWhen(s.created_at_ms)})`,
              }))}
            />
          </Field>
          <Field label="Schema">
            <Select
              value={picker.schema}
              onChange={(schema) => onChange({ ...picker, schema })}
              placeholder="Select…"
              options={snapshotSchemas.map((s) => ({ value: s, label: s }))}
            />
          </Field>
        </>
      )}
    </div>
  );
}

function SnapshotManager({
  connections,
  byId,
  snapshots,
  onChanged,
  onError,
}: {
  connections: ConnList;
  byId: ConnState;
  snapshots: SchemaSnapshotMeta[];
  onChanged: () => void;
  onError: (message: string) => void;
}) {
  const connectedConns = connections.filter(
    (c) => byId[c.id]?.status === "connected",
  );
  const [connectionId, setConnectionId] = useState("");
  const [database, setDatabase] = useState("");
  const [saving, setSaving] = useState(false);

  const databases = byId[connectionId]?.databases ?? [];
  const canSave = !!connectionId && !!database && !saving;

  async function save() {
    setSaving(true);
    try {
      await unwrap(commands.saveSchemaSnapshot(connectionId, database));
      onChanged();
    } catch (err) {
      onError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }

  async function remove(id: string) {
    try {
      await unwrap(commands.deleteSchemaSnapshot(id));
      onChanged();
    } catch (err) {
      onError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className="flex flex-col gap-2 rounded-[6px] border border-border-default bg-bg-2 p-2.5">
      <span className="text-[11.5px] font-semibold text-fg-0">Snapshots</span>
      <div className="flex items-end gap-2">
        <Field label="Connection" className="flex-1">
          <Select
            value={connectionId}
            onChange={(id) => {
              setConnectionId(id);
              setDatabase("");
            }}
            placeholder={connectedConns.length ? "Select…" : "No connected databases"}
            options={connectedConns.map((c) => ({ value: c.id, label: c.name }))}
          />
        </Field>
        <Field label="Database" className="flex-1">
          <Select
            value={database}
            onChange={setDatabase}
            placeholder="Select…"
            options={databases.map((d) => ({ value: d.name, label: d.name }))}
          />
        </Field>
        <button
          type="button"
          disabled={!canSave}
          onClick={() => void save()}
          className="inline-flex h-[26px] items-center gap-1.5 rounded-[4px] border border-border-default bg-bg-1 px-2.5 text-[11.5px] text-fg-1 hover:bg-bg-3 disabled:cursor-not-allowed disabled:opacity-60"
        >
          <Icon.download size={11} />
          {saving ? "Saving…" : "Save snapshot"}
        </button>
      </div>
      {snapshots.length > 0 && (
        <div className="max-h-[120px] overflow-y-auto rounded-[4px] border border-border-divider">
          {snapshots.map((s) => (
            <div
              key={s.id}
              className="flex items-center gap-2 border-b border-border-divider px-2 py-1 text-[11px] last:border-b-0"
            >
              <span className="min-w-0 flex-1 truncate text-fg-1">{s.label}</span>
              <span className="shrink-0 text-[10px] text-fg-3">
                {s.table_count} tables · {formatWhen(s.created_at_ms)}
              </span>
              <button
                type="button"
                className="icon-btn h-[18px] w-[18px]"
                title="Delete snapshot"
                onClick={() => void remove(s.id)}
              >
                <Icon.trash size={11} />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ModeTab({
  active,
  label,
  onClick,
}: {
  active: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={
        "rounded-[4px] px-2 py-0.5 text-[11px] " +
        (active
          ? "bg-accent-soft text-accent"
          : "text-fg-2 hover:bg-bg-3 hover:text-fg-0")
      }
    >
      {label}
    </button>
  );
}

function Field({
  label,
  className = "",
  children,
}: {
  label: string;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <label className={"flex flex-col gap-0.5 " + className}>
      <span className="text-[10px] uppercase tracking-[0.05em] text-fg-3">
        {label}
      </span>
      {children}
    </label>
  );
}

function Select({
  value,
  onChange,
  options,
  placeholder,
}: {
  value: string;
  onChange: (value: string) => void;
  options: { value: string; label: string }[];
  placeholder: string;
}) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="h-[26px] rounded-[4px] border border-border-default bg-bg-1 px-1.5 text-[11.5px] text-fg-1 outline-none focus:border-accent-line"
    >
      <option value="">{placeholder}</option>
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

function toSource(picker: Picker): SchemaSource | null {
  if (picker.mode === "live") {
    if (!picker.connectionId || !picker.database || !picker.schema) return null;
    return {
      kind: "live",
      connection_id: picker.connectionId,
      database: picker.database,
      schema: picker.schema,
      label: null,
    };
  }
  if (!picker.snapshotId || !picker.schema) return null;
  return {
    kind: "snapshot",
    id: picker.snapshotId,
    schema: picker.schema,
    label: null,
  };
}

function formatWhen(ms: number): string {
  if (!ms) return "—";
  try {
    return new Date(ms).toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return "—";
  }
}
