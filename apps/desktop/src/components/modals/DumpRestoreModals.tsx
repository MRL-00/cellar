import { useMemo, useState } from "react";
import { Channel } from "@tauri-apps/api/core";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import {
  commands,
  isTauri,
  unwrap,
  type DumpContents,
  type DumpScope,
  type TransferProgress,
} from "@cellar/ipc";

import { useConnections } from "../../state/connections";
import { Icon } from "../icons";
import { Modal } from "./Modal";
import { ED_RUN_PRIMARY, ED_RUN_SUBTLE } from "./settingsPrimitives";

/** What the sidebar hands the dump modal: a connection + a table/schema scope. */
export interface DumpPreset {
  connectionId: string;
  scope: DumpScope;
}

/** What the sidebar hands the restore modal: a connection + a target database. */
export interface RestorePreset {
  connectionId: string;
  database: string;
}

type Phase = "idle" | "running" | "done" | "cancelled" | "error";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = n / 1024;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i++;
  }
  return `${value.toFixed(value >= 10 || i === 0 ? 0 : 1)} ${units[i]}`;
}

/** Banner shown when the operation targets a production-tagged connection. */
function ProdWarning({ connectionId }: { connectionId: string }) {
  const config = useConnections((s) =>
    s.connections.find((c) => c.id === connectionId),
  );
  if (config?.env_tag !== "prod") return null;
  return (
    <div className="mb-3 flex items-start gap-2 rounded-[4px] border border-warn/40 bg-warn/10 px-3 py-2 text-[11px] text-warn">
      <Icon.warn size={13} />
      <span>
        <strong>{config.name}</strong> is tagged <code>prod</code>. Double-check
        the target before running.
      </span>
    </div>
  );
}

function StatusLine({
  phase,
  bytes,
  error,
  verb,
}: {
  phase: Phase;
  bytes: number;
  error: string | null;
  verb: string;
}) {
  if (phase === "idle") return null;
  if (phase === "error") {
    return (
      <pre className="mt-3 max-h-40 overflow-auto whitespace-pre-wrap rounded-[4px] border border-warn/40 bg-warn/10 px-3 py-2 font-mono text-[10.5px] text-warn">
        {error}
      </pre>
    );
  }
  const label =
    phase === "running"
      ? `${verb}… ${formatBytes(bytes)}`
      : phase === "cancelled"
        ? "Cancelled"
        : `Done · ${formatBytes(bytes)}`;
  return (
    <div className="mt-3">
      <div className="mb-1 flex items-center justify-between font-mono text-[10.5px] text-fg-2">
        <span>{label}</span>
      </div>
      <div className="h-1.5 overflow-hidden rounded-full bg-bg-3">
        <div
          className={
            "h-full bg-accent " +
            (phase === "running" ? "w-1/3 animate-sb-pulse" : "w-full")
          }
        />
      </div>
    </div>
  );
}

export function DumpModal({
  preset,
  onClose,
}: {
  preset: DumpPreset;
  onClose: () => void;
}) {
  const { connectionId, scope } = preset;
  const [contents, setContents] = useState<DumpContents>("both");
  const [destPath, setDestPath] = useState<string | null>(null);
  const [phase, setPhase] = useState<Phase>("idle");
  const [bytes, setBytes] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [opId, setOpId] = useState<string | null>(null);

  const title =
    scope.kind === "table"
      ? `${scope.schema}.${scope.table}`
      : `schema ${scope.schema}`;
  const defaultName =
    scope.kind === "table"
      ? `${scope.schema}.${scope.table}.sql`
      : `${scope.schema}.sql`;

  const chooseDest = async () => {
    if (!isTauri) return;
    const path = await saveDialog({
      defaultPath: defaultName,
      filters: [{ name: "SQL", extensions: ["sql"] }],
    });
    if (path) setDestPath(path);
  };

  const run = async () => {
    if (!destPath) return;
    const channel = new Channel<TransferProgress>();
    channel.onmessage = (p) => setBytes(p.bytes);
    const id = crypto.randomUUID();
    setOpId(id);
    setPhase("running");
    setBytes(0);
    setError(null);
    try {
      const summary = await unwrap(
        commands.dumpPostgres(connectionId, scope, contents, destPath, id, channel),
      );
      setBytes(summary.bytes);
      setPhase("done");
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.toLowerCase().includes("cancelled")) setPhase("cancelled");
      else {
        setError(msg);
        setPhase("error");
      }
    } finally {
      setOpId(null);
    }
  };

  const running = phase === "running";

  return (
    <Modal onClose={onClose} width={520}>
      <Header icon={<Icon.download size={14} />} title={`Dump ${title}`} onClose={onClose} />
      <div className="flex-1 overflow-y-auto px-4 py-3.5">
        <ProdWarning connectionId={connectionId} />

        <Field label="Contents">
          <Segment
            options={[
              { value: "both", label: "Schema + data" },
              { value: "schema-only", label: "Schema only" },
              { value: "data-only", label: "Data only" },
            ]}
            value={contents}
            onChange={(v) => setContents(v as DumpContents)}
            disabled={running}
          />
        </Field>

        <Field label="Format">
          <span className="font-mono text-[11px] text-fg-2">
            plain SQL <span className="text-fg-3">(.sql)</span>
          </span>
        </Field>

        <Field label="Destination">
          <button
            type="button"
            className={ED_RUN_SUBTLE}
            onClick={() => void chooseDest()}
            disabled={running || !isTauri}
          >
            <Icon.fileText size={11} />
            <span>Choose file…</span>
          </button>
        </Field>
        {destPath && (
          <p className="mt-1 break-all font-mono text-[10.5px] text-fg-3">
            {destPath}
          </p>
        )}

        <StatusLine phase={phase} bytes={bytes} error={error} verb="Dumping" />
      </div>

      <Footer
        running={running}
        primaryLabel="Dump"
        primaryDisabled={!destPath || !isTauri}
        onPrimary={() => void run()}
        onCancelOp={() => opId && void commands.cancelDump(opId)}
        onClose={onClose}
        done={phase === "done"}
      />
    </Modal>
  );
}

export function RestoreModal({
  preset,
  onClose,
}: {
  preset: RestorePreset;
  onClose: () => void;
}) {
  const { connectionId, database } = preset;
  const [sourcePath, setSourcePath] = useState<string | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [phase, setPhase] = useState<Phase>("idle");
  const [bytes, setBytes] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [opId, setOpId] = useState<string | null>(null);

  const chooseSource = async () => {
    if (!isTauri) return;
    const path = await openDialog({
      multiple: false,
      filters: [{ name: "SQL", extensions: ["sql"] }],
    });
    if (typeof path === "string") setSourcePath(path);
  };

  const run = async () => {
    if (!sourcePath || !confirmed) return;
    const channel = new Channel<TransferProgress>();
    channel.onmessage = (p) => setBytes(p.bytes);
    const id = crypto.randomUUID();
    setOpId(id);
    setPhase("running");
    setBytes(0);
    setError(null);
    try {
      const summary = await unwrap(
        commands.restorePostgres(connectionId, database, sourcePath, id, channel),
      );
      setBytes(summary.bytes);
      setPhase("done");
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.toLowerCase().includes("cancelled")) setPhase("cancelled");
      else {
        setError(msg);
        setPhase("error");
      }
    } finally {
      setOpId(null);
    }
  };

  const running = phase === "running";

  return (
    <Modal onClose={onClose} width={520}>
      <Header icon={<Icon.upload size={14} />} title={`Restore into ${database}`} onClose={onClose} />
      <div className="flex-1 overflow-y-auto px-4 py-3.5">
        <ProdWarning connectionId={connectionId} />

        <Field label="Dump file">
          <button
            type="button"
            className={ED_RUN_SUBTLE}
            onClick={() => void chooseSource()}
            disabled={running || !isTauri}
          >
            <Icon.fileText size={11} />
            <span>Choose dump file…</span>
          </button>
        </Field>
        {sourcePath && (
          <p className="mt-1 break-all font-mono text-[10.5px] text-fg-3">
            {sourcePath}
          </p>
        )}

        <label className="mt-3 flex cursor-pointer items-start gap-2 rounded-[4px] border border-border-default bg-bg-inset px-3 py-2 text-[11px] text-fg-1">
          <input
            type="checkbox"
            checked={confirmed}
            onChange={(e) => setConfirmed(e.target.checked)}
            disabled={running}
            className="mt-px h-3.5 w-3.5 accent-[var(--accent)]"
          />
          <span>
            I understand this runs the file's statements against{" "}
            <strong>{database}</strong> in a single transaction and may modify or
            overwrite existing data.
          </span>
        </label>

        <StatusLine phase={phase} bytes={bytes} error={error} verb="Restoring" />
      </div>

      <Footer
        running={running}
        primaryLabel="Restore"
        primaryDisabled={!sourcePath || !confirmed || !isTauri}
        onPrimary={() => void run()}
        onCancelOp={() => opId && void commands.cancelDump(opId)}
        onClose={onClose}
        done={phase === "done"}
        danger
      />
    </Modal>
  );
}

function Header({
  icon,
  title,
  onClose,
}: {
  icon: React.ReactNode;
  title: string;
  onClose: () => void;
}) {
  return (
    <div className="flex h-[38px] shrink-0 items-center justify-between border-b border-border-default pl-3.5 pr-2">
      <div className="flex items-center gap-2">
        <span className="inline-flex text-accent">{icon}</span>
        <span className="whitespace-nowrap text-[12.5px] font-semibold text-fg-0">
          {title}
        </span>
      </div>
      <button className="icon-btn" onClick={onClose} title="Close">
        <Icon.close size={13} />
      </button>
    </div>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="mb-3 flex items-center gap-3">
      <span className="w-[88px] shrink-0 text-[11px] text-fg-2">{label}</span>
      <div className="flex min-w-0 flex-1 items-center gap-2">{children}</div>
    </div>
  );
}

function Segment({
  options,
  value,
  onChange,
  disabled,
}: {
  options: { value: string; label: string }[];
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}) {
  return (
    <div className="inline-flex rounded-[5px] border border-border-default bg-bg-inset p-0.5">
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          disabled={disabled}
          onClick={() => onChange(o.value)}
          className={
            "rounded-[3px] px-2 py-1 text-[11px] transition-colors disabled:cursor-not-allowed " +
            (value === o.value
              ? "bg-accent text-accent-fg"
              : "text-fg-2 hover:text-fg-0")
          }
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

function Footer({
  running,
  primaryLabel,
  primaryDisabled,
  onPrimary,
  onCancelOp,
  onClose,
  done,
  danger,
}: {
  running: boolean;
  primaryLabel: string;
  primaryDisabled: boolean;
  onPrimary: () => void;
  onCancelOp: () => void;
  onClose: () => void;
  done: boolean;
  danger?: boolean;
}) {
  return (
    <div className="flex h-11 shrink-0 items-center justify-end gap-2 border-t border-border-default bg-bg-2 px-3">
      <button className={ED_RUN_SUBTLE} onClick={onClose}>
        {done ? "Close" : "Cancel"}
      </button>
      {running ? (
        <button className={ED_RUN_SUBTLE} onClick={onCancelOp}>
          <Icon.stop size={11} />
          <span>Stop</span>
        </button>
      ) : (
        <button
          className={
            (danger ? ED_RUN_SUBTLE + " !text-warn" : ED_RUN_PRIMARY) +
            " disabled:cursor-not-allowed disabled:opacity-40"
          }
          onClick={onPrimary}
          disabled={primaryDisabled}
        >
          <span>{primaryLabel}</span>
        </button>
      )}
    </div>
  );
}
