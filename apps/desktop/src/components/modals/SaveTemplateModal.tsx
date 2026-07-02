import { useState } from "react";
import { commands, unwrap } from "@cellar/ipc";

import { Icon } from "../icons";
import { ED_RUN_PRIMARY, ED_RUN_SUBTLE } from "./settingsPrimitives";
import { Modal } from "./Modal";

/**
 * Save the current query to the local template library (`~/.cellar/queries/`).
 * Local files only — there is no server-side template storage.
 */
export function SaveTemplateModal({
  sql,
  defaultName,
  onClose,
}: {
  sql: string;
  defaultName: string;
  onClose: () => void;
}) {
  const [name, setName] = useState(defaultName);
  const [description, setDescription] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const canSave = name.trim().length > 0 && sql.trim().length > 0 && !saving;

  const onSave = async () => {
    if (!canSave) return;
    setSaving(true);
    setError(null);
    try {
      await unwrap(
        commands.saveQueryTemplate({
          name: name.trim(),
          description: description.trim(),
          sql,
        }),
      );
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setSaving(false);
    }
  };

  return (
    <Modal onClose={onClose} width={520}>
      <div className="flex h-[38px] shrink-0 items-center justify-between border-b border-border-default pl-3.5 pr-2">
        <div className="flex items-center gap-2">
          <span className="inline-flex text-accent">
            <Icon.star size={14} />
          </span>
          <span className="whitespace-nowrap text-sm font-semibold text-fg-0">
            Save query template
          </span>
        </div>
        <button className="icon-btn" onClick={onClose} title="Close">
          <Icon.close size={13} />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-4 py-3.5">
        <label className="mb-1 block text-sm font-medium text-fg-1">Name</label>
        <input
          className="mb-3 w-full rounded-[5px] border border-border-default bg-bg-inset px-2.5 py-1.5 text-sm text-fg-0 outline-none focus:border-accent"
          value={name}
          autoFocus
          spellCheck={false}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if ((e.metaKey || e.ctrlKey) && e.key === "Enter") void onSave();
          }}
          placeholder="Recent orders by region"
        />

        <label className="mb-1 block text-sm font-medium text-fg-1">
          Description <span className="text-fg-3">(optional)</span>
        </label>
        <textarea
          className="mb-3 h-16 w-full resize-none rounded-[5px] border border-border-default bg-bg-inset px-2.5 py-1.5 text-sm text-fg-0 outline-none focus:border-accent"
          value={description}
          spellCheck={false}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="What this query is for…"
        />

        <label className="mb-1 block text-sm font-medium text-fg-1">SQL</label>
        <pre className="m-0 max-h-32 overflow-auto rounded-[5px] border border-border-default bg-bg-inset px-2.5 py-1.5 font-mono text-sm leading-[1.5] text-fg-2 whitespace-pre-wrap">
          {sql.trim()}
        </pre>

        {error && (
          <p className="mt-2 mb-0 text-sm text-delete">{error}</p>
        )}
      </div>

      <div className="flex h-11 shrink-0 items-center justify-end gap-2 border-t border-border-default bg-bg-2 px-3">
        <button className={ED_RUN_SUBTLE} onClick={onClose}>
          Cancel
        </button>
        <button
          className={ED_RUN_PRIMARY + " disabled:cursor-not-allowed disabled:opacity-40"}
          onClick={() => void onSave()}
          disabled={!canSave}
        >
          <Icon.star size={11} />
          <span>{saving ? "Saving…" : "Save template"}</span>
        </button>
      </div>
    </Modal>
  );
}
