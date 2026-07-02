import { useState } from "react";
import type { Schema } from "@cellar/ipc";

import { Icon } from "./icons";
import { schemaHasObjects } from "./SidebarTree";

export type SchemaManagerState = {
  connectionId: string;
  database: string;
  schemas: Schema[];
};

export function SchemaVisibilityManager({
  state,
  prefs,
  onClose,
  onChangeVisible,
}: {
  state: SchemaManagerState;
  prefs: { hidden: string[]; showHidden: boolean };
  onClose: () => void;
  onChangeVisible: (
    connectionId: string,
    database: string,
    schemas: Schema[],
    visible: Set<string>,
  ) => void;
}) {
  const [filter, setFilter] = useState("");
  const hidden = new Set(prefs.hidden);
  const hasNonEmpty = state.schemas.some(schemaHasObjects);
  const visible = new Set(
    state.schemas
      .filter((schema) => {
        if (hidden.has(schema.name)) return false;
        if (hasNonEmpty && !prefs.showHidden && !schemaHasObjects(schema)) {
          return false;
        }
        return true;
      })
      .map((schema) => schema.name),
  );
  const filtered = state.schemas.filter((schema) =>
    schema.name.toLowerCase().includes(filter.trim().toLowerCase()),
  );

  const update = (next: Set<string>) => {
    onChangeVisible(state.connectionId, state.database, state.schemas, next);
  };

  const setOne = (schema: string, checked: boolean) => {
    const next = new Set(visible);
    if (checked) next.add(schema);
    else next.delete(schema);
    update(next);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/35">
      <div className="flex max-h-[70vh] w-[420px] flex-col overflow-hidden rounded-[8px] border border-border-default bg-bg-1 shadow-xl">
        <div className="flex h-10 shrink-0 items-center justify-between border-b border-border-default px-3">
          <div className="min-w-0">
            <div className="text-[12.5px] font-semibold text-fg-0">
              Visible schemas
            </div>
            <div className="truncate text-[10.5px] text-fg-3">
              {state.database}
            </div>
          </div>
          <button className="icon-btn" title="Close" onClick={onClose}>
            <Icon.close size={12} />
          </button>
        </div>

        <div className="border-b border-border-default px-3 py-2">
          <div className="mb-2 flex min-h-7 items-center gap-1.5 rounded-[4px] border border-border-default bg-bg-inset px-2 focus-within:border-accent-line">
            <Icon.search size={11} style={{ color: "var(--fg-3)" }} />
            <input
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder="Filter schemas..."
              className="min-w-0 flex-1 border-none bg-transparent py-1 text-[11.5px] text-fg-0 outline-none placeholder:text-fg-3"
            />
          </div>
          <div className="flex flex-wrap gap-1">
            <SchemaAction
              label="All"
              onClick={() =>
                update(new Set(state.schemas.map((schema) => schema.name)))
              }
            />
            <SchemaAction label="None" onClick={() => update(new Set())} />
            <SchemaAction
              label="Non-empty"
              onClick={() =>
                update(
                  new Set(
                    state.schemas
                      .filter(schemaHasObjects)
                      .map((schema) => schema.name),
                  ),
                )
              }
            />
            <SchemaAction
              label="Empty"
              onClick={() =>
                update(
                  new Set(
                    state.schemas
                      .filter((schema) => !schemaHasObjects(schema))
                      .map((schema) => schema.name),
                  ),
                )
              }
            />
          </div>
        </div>

        <div className="flex-1 overflow-y-auto py-1">
          {filtered.map((schema) => {
            const checked = visible.has(schema.name);
            const count = schema.tables.length + schema.views.length;
            return (
              <label
                key={schema.name}
                className="flex h-7 cursor-pointer items-center gap-2 px-3 text-[11.5px] text-fg-1 hover:bg-bg-2"
              >
                <input
                  type="checkbox"
                  checked={checked}
                  onChange={(e) => setOne(schema.name, e.target.checked)}
                  className="h-3.5 w-3.5 accent-[var(--accent)]"
                />
                <Icon.schema size={12} />
                <span className="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
                  {schema.name}
                </span>
                <span className="text-[10px] tabular-nums text-fg-3">
                  {count}
                </span>
              </label>
            );
          })}
        </div>

        <div className="flex h-9 shrink-0 items-center justify-between border-t border-border-default px-3">
          <span className="text-[10.5px] tabular-nums text-fg-3">
            {visible.size}/{state.schemas.length} visible
          </span>
          <button
            className="h-[24px] rounded-[4px] bg-accent px-2.5 text-[11px] font-medium text-accent-fg"
            onClick={onClose}
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
}

function SchemaAction({
  label,
  onClick,
}: {
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className="h-5 rounded-[3px] border border-border-default bg-bg-2 px-2 text-[10.5px] text-fg-1 hover:border-border-strong hover:text-fg-0"
      onClick={onClick}
    >
      {label}
    </button>
  );
}
