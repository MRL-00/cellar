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

/* Demo changeset matching the design's commit preview. */
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
    sqlLines.push(
      `INSERT INTO public.${table} (${cols.join(", ")})`,
    );
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
      <div className="cd-head">
        <div className="cd-head-left">
          <span className="cd-head-icon">
            <Icon.commit size={14} />
          </span>
          <span className="cd-head-title">Review &amp; commit</span>
          <span className="cm-target">
            public.{table} <span style={{ color: "var(--fg-3)" }}>·</span>{" "}
            shop-eu (prod)
          </span>
        </div>
        <button className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

      <div className="cm-summary">
        <div className="cm-summary-item">
          <span
            className="cm-summary-icon"
            style={{ background: "var(--insert-bg)", color: "var(--insert)" }}
          >
            <Icon.plus size={11} />
          </span>
          <span className="cm-summary-n tnum mono">{inserts.length}</span>
          <span className="cm-summary-label">
            insert{inserts.length === 1 ? "" : "s"}
          </span>
        </div>
        <div className="cm-summary-item">
          <span
            className="cm-summary-icon"
            style={{ background: "var(--update-bg)", color: "var(--update)" }}
          >
            <Icon.diff size={11} />
          </span>
          <span className="cm-summary-n tnum mono">{updates.length}</span>
          <span className="cm-summary-label">
            update{updates.length === 1 ? "" : "s"}
          </span>
        </div>
        <div className="cm-summary-item">
          <span
            className="cm-summary-icon"
            style={{ background: "var(--delete-bg)", color: "var(--delete)" }}
          >
            <Icon.close size={11} />
          </span>
          <span className="cm-summary-n tnum mono">{deletes.length}</span>
          <span className="cm-summary-label">
            delete{deletes.length === 1 ? "" : "s"}
          </span>
        </div>
        <div className="cm-summary-spacer" />
        <span className="cm-summary-tx mono">
          <Icon.bracket size={10} />
          <span>BEGIN … COMMIT — atomic</span>
        </span>
      </div>

      <div className="cm-body">
        <div className="cm-changes">
          <div className="cm-changes-head">
            <span>Changes</span>
            <span className="cm-changes-count mono">{entries.length}</span>
          </div>
          <div className="cm-changes-list">
            {updates.map(([id, c]) => (
              <div key={id} className="cm-change cm-change-update">
                <span className="cm-change-tag">UPDATE</span>
                <div className="cm-change-body">
                  <div className="cm-change-row">
                    <span className="cm-change-id">{c.row.order_number}</span>
                    <span style={{ color: "var(--fg-3)" }}>·</span>
                    <span
                      className="mono"
                      style={{ color: "var(--fg-3)", fontSize: 10 }}
                    >
                      {id.slice(0, 8)}
                    </span>
                  </div>
                  {Object.entries(c.edits).map(([col, e]) => (
                    <div key={col} className="cm-edit">
                      <span className="cm-edit-col">{col}</span>
                      <span className="cm-edit-from">{formatVal(e.from)}</span>
                      <Icon.chevronRight size={10} stroke="var(--fg-3)" />
                      <span className="cm-edit-to">{formatVal(e.to)}</span>
                    </div>
                  ))}
                </div>
              </div>
            ))}
            {inserts.map(([id, c]) => (
              <div key={id} className="cm-change cm-change-insert">
                <span className="cm-change-tag">INSERT</span>
                <div className="cm-change-body">
                  <div className="cm-change-row">
                    <span className="cm-change-id">
                      {String((c.row as { order_number?: string }).order_number ?? "new")}
                    </span>
                    <span style={{ color: "var(--fg-3)" }}>·</span>
                    <span
                      className="mono"
                      style={{ color: "var(--fg-3)", fontSize: 10 }}
                    >
                      new
                    </span>
                  </div>
                  <div
                    className="cm-edit"
                    style={{
                      color: "var(--fg-2)",
                      gridTemplateColumns: "1fr",
                    }}
                  >
                    new row · {Object.keys(c.row).length} columns set
                  </div>
                </div>
              </div>
            ))}
            {deletes.map(([id, c]) => (
              <div key={id} className="cm-change cm-change-delete">
                <span className="cm-change-tag">DELETE</span>
                <div className="cm-change-body">
                  <div className="cm-change-row">
                    <span
                      className="cm-change-id"
                      style={{ textDecoration: "line-through" }}
                    >
                      {c.row.order_number}
                    </span>
                    <span style={{ color: "var(--fg-3)" }}>·</span>
                    <span
                      className="mono"
                      style={{ color: "var(--fg-3)", fontSize: 10 }}
                    >
                      {id.slice(0, 8)}
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="cm-sql">
          <div className="cm-sql-head">
            <span>Generated SQL</span>
            <div className="cm-sql-head-right">
              <button className="cd-pick">
                <Icon.edit size={11} />
                <span>Edit</span>
              </button>
              <button className="cd-pick">
                <Icon.copy size={11} />
                <span>Copy</span>
              </button>
            </div>
          </div>
          <div className="cm-sql-body mono">
            {lines.map((toks, i) => (
              <div key={i} className="cm-sql-line">
                <span className="cm-sql-ln">{i + 1}</span>
                <span className="cm-sql-text">{renderTokens(toks)}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="cd-foot">
        <div className="cd-foot-left">
          <label className="cd-check">
            <input type="checkbox" defaultChecked /> Rollback on error
          </label>
          <label className="cd-check">
            <input type="checkbox" defaultChecked /> Confirm if rows affected
            &gt; 100
          </label>
          <span className="cm-foot-warn">
            <Icon.warn size={10} stroke="var(--warn)" />
            <span style={{ color: "var(--warn)" }}>prod</span>
            <span style={{ color: "var(--fg-2)" }}>
              · you'll be asked to type the connection name
            </span>
          </span>
        </div>
        <div className="cd-foot-right">
          <button className="ed-run subtle" onClick={onClose}>
            Cancel
          </button>
          <button className="ed-run subtle">
            <Icon.undo size={11} />
            <span>Save as migration</span>
          </button>
          <button className="ed-run primary danger">
            <Icon.commit size={11} />
            <span>Commit transaction</span>
          </button>
        </div>
      </div>
    </Modal>
  );
}
