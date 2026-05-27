import { Icon } from "../icons";
import { Modal } from "./Modal";
import { tokenizeSql, tokensToLines, renderTokens } from "../../lib/sqlTokens";

type Edit = { from: string | number | null; to: string | number | null };
type UpdateChange = {
  kind: "update";
  row: { order_number: string };
  edits: Record<string, Edit>;
};
type InsertChange = { kind: "insert"; row: Record<string, unknown> };
type DeleteChange = { kind: "delete"; row: { order_number: string } };
export type Change = UpdateChange | InsertChange | DeleteChange;

export type ChangeSet = Record<string, Change>;

export const SAMPLE_CHANGES: ChangeSet = {
  "0a14b9b2-7f6c-4f2d-8a17-3e9f30caa101": {
    kind: "update",
    row: { order_number: "EU-0184237" },
    edits: {
      status: { from: "paid", to: "fulfilled" },
      shipping_method: { from: "standard", to: "express" },
    },
  },
  "2c4dd1a3-9914-4d12-9f8b-1c0e0a3bbf24": {
    kind: "update",
    row: { order_number: "EU-0184244" },
    edits: {
      status: { from: "paid", to: "cancelled" },
      notes: { from: null, to: "customer requested cancel" },
    },
  },
  "9f1c2c50-22d0-43a8-9d12-aa3f6e5210ef": {
    kind: "insert",
    row: {
      id: "9f1c2c50-22d0-43a8-9d12-aa3f6e5210ef",
      order_number: "EU-0184902",
      customer_id: "1d4e2-...",
      status: "pending",
      total_eur: 84.5,
      currency: "EUR",
      channel: "web",
      country: "DE",
    },
  },
  "ba27e1f8-1d61-4b09-bc44-9e102239f48a": {
    kind: "delete",
    row: { order_number: "EU-0184121" },
  },
};

function formatVal(v: unknown): string {
  if (v === null || v === undefined) return "NULL";
  if (typeof v === "number") return String(v);
  return "'" + v + "'";
}

const ED_RUN_BASE =
  "inline-flex h-[26px] items-center gap-[5px] whitespace-nowrap rounded-[4px] border border-transparent px-2.5 text-[11.5px] font-medium text-fg-1 transition-[background,color,border-color,filter] duration-[120ms]";

const ED_RUN_SUBTLE =
  ED_RUN_BASE +
  " bg-transparent border-border-default hover:bg-bg-3 hover:border-border-strong hover:text-fg-0";

const ED_RUN_DANGER =
  ED_RUN_BASE +
  " bg-delete text-white hover:brightness-[1.07]";

export function CommitModal({
  onClose,
  changes = SAMPLE_CHANGES,
  table = "orders",
}: {
  onClose: () => void;
  changes?: ChangeSet;
  table?: string;
}) {
  const entries = Object.entries(changes);
  const updates = entries.filter(([, c]) => c.kind === "update") as [
    string,
    UpdateChange,
  ][];
  const inserts = entries.filter(([, c]) => c.kind === "insert") as [
    string,
    InsertChange,
  ][];
  const deletes = entries.filter(([, c]) => c.kind === "delete") as [
    string,
    DeleteChange,
  ][];

  const sqlLines: string[] = ["BEGIN;", ""];
  updates.forEach(([id, c]) => {
    sqlLines.push(`-- ${c.row.order_number} · updated by alice@laptop`);
    sqlLines.push(`UPDATE public.${table}`);
    const sets = Object.entries(c.edits).map(
      ([col, e]) => `  ${col} = ${formatVal(e.to)}`,
    );
    sqlLines.push("SET");
    sqlLines.push(sets.join(",\n"));
    sqlLines.push(`WHERE id = '${id}';`);
    sqlLines.push("");
  });
  inserts.forEach(([id, c]) => {
    const cols = Object.keys(c.row);
    const vals = cols.map((k) => formatVal(c.row[k]));
    sqlLines.push(`INSERT INTO public.${table} (${cols.join(", ")})`);
    sqlLines.push(`VALUES (${vals.join(", ")});`);
    sqlLines.push("");
    void id;
  });
  deletes.forEach(([id, c]) => {
    sqlLines.push(`-- ${c.row.order_number} · marked for deletion`);
    sqlLines.push(`DELETE FROM public.${table} WHERE id = '${id}';`);
    sqlLines.push("");
  });
  sqlLines.push("COMMIT;");
  const lines = tokensToLines(tokenizeSql(sqlLines.join("\n")));

  return (
    <Modal onClose={onClose} width={880}>
      <div className="flex h-[38px] shrink-0 items-center justify-between border-b border-border-default pl-3.5 pr-2">
        <div className="flex items-center gap-2">
          <span className="inline-flex text-accent">
            <Icon.commit size={14} />
          </span>
          <span className="whitespace-nowrap text-[12.5px] font-semibold text-fg-0">
            Review &amp; commit
          </span>
          <span className="ml-1 border-l border-border-divider pl-1.5 font-mono text-[11px] text-fg-2">
            public.{table} <span style={{ color: "var(--fg-3)" }}>·</span>{" "}
            shop-eu (prod)
          </span>
        </div>
        <button className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

      <div className="flex shrink-0 items-center gap-4 border-b border-border-default bg-bg-2 px-4 py-2.5">
        <SummaryItem
          icon={<Icon.plus size={11} />}
          bg="var(--insert-bg)"
          color="var(--insert)"
          n={inserts.length}
          label={`insert${inserts.length === 1 ? "" : "s"}`}
        />
        <SummaryItem
          icon={<Icon.diff size={11} />}
          bg="var(--update-bg)"
          color="var(--update)"
          n={updates.length}
          label={`update${updates.length === 1 ? "" : "s"}`}
        />
        <SummaryItem
          icon={<Icon.close size={11} />}
          bg="var(--delete-bg)"
          color="var(--delete)"
          n={deletes.length}
          label={`delete${deletes.length === 1 ? "" : "s"}`}
        />
        <div className="flex-1" />
        <span className="inline-flex items-center gap-1.5 whitespace-nowrap rounded-[4px] bg-bg-inset px-2 py-1 font-mono text-[10.5px] text-fg-2">
          <Icon.bracket size={10} />
          <span>BEGIN … COMMIT — atomic</span>
        </span>
      </div>

      <div className="grid min-h-0 flex-1 overflow-hidden grid-cols-[320px_1fr]">
        <div className="flex min-h-0 flex-col border-r border-border-default">
          <div className="flex h-[26px] shrink-0 items-center gap-1.5 border-b border-border-divider px-3 text-[10px] font-semibold uppercase tracking-[0.05em] text-fg-3">
            <span>Changes</span>
            <span className="rounded-[8px] bg-bg-2 px-1.5 py-px font-mono text-[10px] text-fg-2">
              {entries.length}
            </span>
          </div>
          <div className="flex-1 overflow-y-auto py-1.5">
            {updates.map(([id, c]) => (
              <ChangeRow key={id} tag="UPDATE" tagBg="var(--update-bg)" tagColor="var(--update)">
                <div className="flex items-center gap-1.5 text-[11px]">
                  <span className="font-mono font-medium text-fg-0">
                    {c.row.order_number}
                  </span>
                  <span style={{ color: "var(--fg-3)" }}>·</span>
                  <span className="font-mono" style={{ color: "var(--fg-3)", fontSize: 10 }}>
                    {id.slice(0, 8)}
                  </span>
                </div>
                {Object.entries(c.edits).map(([col, e]) => (
                  <div
                    key={col}
                    className="grid items-center gap-[5px] pl-1 font-mono text-[10.5px] grid-cols-[90px_auto_auto_auto]"
                  >
                    <span className="overflow-hidden text-ellipsis text-fg-2">
                      {col}
                    </span>
                    <span
                      className="overflow-hidden text-ellipsis rounded-[3px] px-1.5 py-px text-delete line-through"
                      style={{
                        background: "var(--delete-bg)",
                        textDecorationColor: "rgba(255, 255, 255, 0.2)",
                      }}
                    >
                      {formatVal(e.from)}
                    </span>
                    <Icon.chevronRight size={10} stroke="var(--fg-3)" />
                    <span className="overflow-hidden text-ellipsis rounded-[3px] bg-accent-soft px-1.5 py-px text-accent">
                      {formatVal(e.to)}
                    </span>
                  </div>
                ))}
              </ChangeRow>
            ))}
            {inserts.map(([id, c]) => (
              <ChangeRow key={id} tag="INSERT" tagBg="var(--insert-bg)" tagColor="var(--insert)">
                <div className="flex items-center gap-1.5 text-[11px]">
                  <span className="font-mono font-medium text-fg-0">
                    {String((c.row as { order_number?: string }).order_number ?? "new")}
                  </span>
                  <span style={{ color: "var(--fg-3)" }}>·</span>
                  <span className="font-mono" style={{ color: "var(--fg-3)", fontSize: 10 }}>
                    new
                  </span>
                </div>
                <div
                  className="pl-1 font-mono text-[10.5px] text-fg-2"
                  style={{ display: "grid", gridTemplateColumns: "1fr", gap: 5 }}
                >
                  new row · {Object.keys(c.row).length} columns set
                </div>
              </ChangeRow>
            ))}
            {deletes.map(([id, c]) => (
              <ChangeRow key={id} tag="DELETE" tagBg="var(--delete-bg)" tagColor="var(--delete)">
                <div className="flex items-center gap-1.5 text-[11px]">
                  <span className="font-mono font-medium text-fg-0 line-through">
                    {c.row.order_number}
                  </span>
                  <span style={{ color: "var(--fg-3)" }}>·</span>
                  <span className="font-mono" style={{ color: "var(--fg-3)", fontSize: 10 }}>
                    {id.slice(0, 8)}
                  </span>
                </div>
              </ChangeRow>
            ))}
          </div>
        </div>

        <div className="flex min-h-0 flex-col bg-bg-inset">
          <div className="flex h-[26px] shrink-0 items-center justify-between border-b border-border-divider bg-bg-1 px-3 text-[10px] font-semibold uppercase tracking-[0.05em] text-fg-3">
            <span>Generated SQL</span>
            <div className="flex gap-1">
              <button className="inline-flex h-[26px] items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-1 hover:bg-bg-3">
                <Icon.edit size={11} />
                <span>Edit</span>
              </button>
              <button className="inline-flex h-[26px] items-center gap-1 rounded-[4px] border border-border-default bg-bg-2 px-2 text-[11px] text-fg-1 hover:bg-bg-3">
                <Icon.copy size={11} />
                <span>Copy</span>
              </button>
            </div>
          </div>
          <div className="flex-1 overflow-auto py-2 font-mono text-[11.5px] leading-[1.55]">
            {lines.map((toks, i) => (
              <div key={i} className="flex px-3">
                <span className="inline-flex w-7 shrink-0 select-none items-center justify-end pr-2.5 font-variant-numeric-tabular text-[10px] text-fg-3 font-mono">
                  {i + 1}
                </span>
                <span className="whitespace-pre font-mono">
                  {renderTokens(toks)}
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="flex h-11 shrink-0 items-center justify-between gap-3 border-t border-border-default bg-bg-2 px-3">
        <div className="flex items-center gap-2">
          <label className="inline-flex cursor-pointer items-center gap-1 text-[11px] text-fg-2">
            <input
              type="checkbox"
              defaultChecked
              className="h-3 w-3"
              style={{ accentColor: "var(--accent)" }}
            />
            Rollback on error
          </label>
          <label className="inline-flex cursor-pointer items-center gap-1 text-[11px] text-fg-2">
            <input
              type="checkbox"
              defaultChecked
              className="h-3 w-3"
              style={{ accentColor: "var(--accent)" }}
            />
            Confirm if rows affected &gt; 100
          </label>
          <span className="ml-2 inline-flex items-center gap-1.5 text-[10.5px]">
            <Icon.warn size={10} stroke="var(--warn)" />
            <span style={{ color: "var(--warn)" }}>prod</span>
            <span style={{ color: "var(--fg-2)" }}>
              · you'll be asked to type the connection name
            </span>
          </span>
        </div>
        <div className="flex items-center gap-2">
          <button className={ED_RUN_SUBTLE} onClick={onClose}>
            Cancel
          </button>
          <button className={ED_RUN_SUBTLE}>
            <Icon.undo size={11} />
            <span>Save as migration</span>
          </button>
          <button
            className={ED_RUN_DANGER}
            style={{
              borderColor: "color-mix(in oklab, var(--delete) 40%, black)",
            }}
          >
            <Icon.commit size={11} />
            <span>Commit transaction</span>
          </button>
        </div>
      </div>
    </Modal>
  );
}

function SummaryItem({
  icon,
  bg,
  color,
  n,
  label,
}: {
  icon: React.ReactNode;
  bg: string;
  color: string;
  n: number;
  label: string;
}) {
  return (
    <div className="flex items-center gap-1.5">
      <span
        className="inline-flex h-[22px] w-[22px] items-center justify-center rounded-[4px]"
        style={{ background: bg, color }}
      >
        {icon}
      </span>
      <span className="font-mono text-[14px] font-semibold text-fg-0 tabular-nums">
        {n}
      </span>
      <span className="text-[11px] text-fg-2">{label}</span>
    </div>
  );
}

function ChangeRow({
  tag,
  tagBg,
  tagColor,
  children,
}: {
  tag: string;
  tagBg: string;
  tagColor: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex gap-2 border-b border-dashed border-border-divider px-3 py-1.5">
      <span
        className="mt-px inline-flex h-[14px] items-center self-start rounded-[3px] px-1 py-px font-mono text-[9.5px] font-semibold tracking-[0.04em]"
        style={{ background: tagBg, color: tagColor }}
      >
        {tag}
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-[3px]">{children}</div>
    </div>
  );
}
