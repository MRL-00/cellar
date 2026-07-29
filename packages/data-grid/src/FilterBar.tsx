import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  type Ref,
} from "react";
import { nativeControl } from "./Cell";
import {
  columnCategory,
  createFilterId,
  defaultFilterValue,
  filterNeedsValue,
  filterOperatorLabel,
  filterValuePreview,
  nextOperatorForColumn,
  operatorsForColumn,
} from "./filters";
import { GridIcon } from "./icons";
import { GridSelect } from "./GridSelect";
import { PresetMenu } from "./PresetMenu";
import type {
  ColumnFilters,
  FilterClause,
  FilterLogic,
  FilterOperator,
  GridColumn,
  SortState,
} from "./types";

const BOOL_FILTER_OPTIONS = [
  { value: "true", label: "true" },
  { value: "false", label: "false" },
] as const;

/**
 * Props for a host-supplied filter value control. Return an element to take over
 * the value slot (e.g. the app calendar picker), or `null` to use the built-in
 * control for that column.
 */
export type FilterValueInputProps = {
  column: GridColumn;
  value: string;
  onChange: (value: string) => void;
  /** Attach to the focusable trigger so the composer can focus the value slot. */
  anchorRef?: Ref<HTMLElement | null>;
};

export type FilterValueRenderer = (
  props: FilterValueInputProps,
) => ReactNode | null;

/**
 * Native date/time control for the filter composer. Always returns a picker for
 * temporal columns (even when the current draft value isn't parseable — the
 * input shows empty until the user picks). Used when the host does not supply
 * {@link FilterBarProps.renderFilterValue}.
 */
function filterTemporalControl(
  column: GridColumn,
  value: string,
): { type: "date" | "datetime-local" | "time"; step?: string; value: string } | null {
  const blank = nativeControl(column, "");
  if (
    !blank ||
    (blank.type !== "date" &&
      blank.type !== "datetime-local" &&
      blank.type !== "time")
  ) {
    return null;
  }
  const parsed = nativeControl(column, value);
  return {
    type: blank.type,
    step: blank.step,
    value: parsed?.value ?? "",
  };
}

type ComposerDraft = {
  id: string | null;
  columnKey: string;
  operator: FilterOperator;
  value: string;
  logic: FilterLogic;
};

export type SavedFilterControls = {
  /** Preset names for this table, in display order. */
  names: readonly string[];
  /**
   * Name of the preset the current toolbar state matches, if any. Shown on the
   * dropdown trigger; hosts typically derive it by comparing state to the
   * saved snapshots, so it clears itself as soon as the user edits anything.
   */
  activeName?: string | null;
  /** Snapshot the current quick filter + chips + sort under `name`. */
  onSave: (name: string) => void;
  onApply: (name: string) => void;
  onDelete: (name: string) => void;
  /** Reset the whole toolbar (quick filter, chips, sort) — unselects. */
  onClear: () => void;
};

export type FilterBarProps = {
  columns: readonly GridColumn[];
  filters: ColumnFilters;
  setFilters: (next: ColumnFilters) => void;
  /**
   * Optional quick filter, pinned at the start of the toolbar. Lives alongside
   * the advanced chips — clearing one never touches the other. Hidden when no
   * `onQuickFilterChange` is supplied.
   */
  quickFilter?: string;
  onQuickFilterChange?: (next: string) => void;
  quickFilterColumn?: string | null;
  onQuickFilterColumnChange?: (next: string | null) => void;
  totalRows: number;
  filteredRows: number;
  serverRows?: number;
  sort?: SortState;
  onSortChange?: (next: SortState) => void;
  /** Saved filter presets. Hidden when omitted. */
  savedFilters?: SavedFilterControls;
  /**
   * Optional override for the filter value control. Same idea as the grid's
   * `renderEditor`: return an element to take over (e.g. the app calendar), or
   * `null` to keep the built-in bool/native-date/text control.
   */
  renderFilterValue?: FilterValueRenderer;
};

export function FilterBar({
  columns,
  filters,
  setFilters,
  quickFilter,
  onQuickFilterChange,
  quickFilterColumn,
  onQuickFilterColumnChange,
  totalRows,
  filteredRows,
  serverRows,
  sort,
  onSortChange,
  savedFilters,
  renderFilterValue,
}: FilterBarProps) {
  const [draft, setDraft] = useState<ComposerDraft | null>(null);
  const columnRef = useRef<HTMLButtonElement | null>(null);
  const valueRef = useRef<HTMLInputElement | null>(null);
  const valueSelectRef = useRef<HTMLButtonElement | null>(null);
  const valueAnchorRef = useRef<HTMLElement | null>(null);
  const columnsByKey = useMemo(
    () => new Map(columns.map((column) => [column.key, column])),
    [columns],
  );

  const focusKey = draft
    ? `${draft.id ?? "new"}:${draft.columnKey}:${draft.operator}`
    : null;

  useEffect(() => {
    if (!draft) return;
    const column = columnsByKey.get(draft.columnKey);
    if (column && filterNeedsValue(draft.operator)) {
      if (columnCategory(column) === "bool") {
        valueSelectRef.current?.focus();
        return;
      }
      if (valueAnchorRef.current) {
        valueAnchorRef.current.focus();
        return;
      }
      valueRef.current?.focus();
      // select() throws on date/time/number inputs — same guard as CellEditor.
      try {
        valueRef.current?.select();
      } catch {
        /* non-text native control */
      }
      return;
    }
    columnRef.current?.focus();
  }, [columnsByKey, focusKey]);

  const openAdd = () => {
    const first = columns[0];
    if (!first) return;
    const operator = operatorsForColumn(first)[0] ?? "equals";
    setDraft({
      id: null,
      columnKey: first.key,
      operator,
      value: defaultFilterValue(first),
      logic: "and",
    });
  };

  const openEdit = (clause: FilterClause) => {
    const column = columnsByKey.get(clause.columnKey);
    setDraft({
      id: clause.id,
      columnKey: clause.columnKey,
      operator: column ? nextOperatorForColumn(column, clause.operator) : clause.operator,
      value: clause.value ?? "",
      logic: clause.logic,
    });
  };

  const applyDraft = () => {
    if (!draft) return;
    const column = columnsByKey.get(draft.columnKey);
    if (!column) return;
    const operator = nextOperatorForColumn(column, draft.operator);
    const needsValue = filterNeedsValue(operator);
    // Bool picker always resolves to true/false; other controls use the trimmed draft.
    const value =
      columnCategory(column) === "bool"
        ? draft.value === "false"
          ? "false"
          : "true"
        : draft.value.trim();
    if (needsValue && value.length === 0) return;

    const clause: FilterClause = {
      id: draft.id ?? createFilterId(),
      columnKey: draft.columnKey,
      operator,
      logic: draft.id
        ? draft.logic
        : filters.length === 0
          ? "and"
          : draft.logic,
      ...(needsValue ? { value } : {}),
    };

    if (draft.id) {
      setFilters(filters.map((item) => (item.id === draft.id ? clause : item)));
    } else {
      setFilters([...filters, clause]);
    }
    setDraft(null);
  };

  const removeFilter = (id: string) => {
    setFilters(filters.filter((clause) => clause.id !== id));
    if (draft?.id === id) setDraft(null);
  };

  const toggleLogic = (id: string) => {
    setFilters(
      filters.map((clause) =>
        clause.id === id
          ? { ...clause, logic: clause.logic === "or" ? "and" : "or" }
          : clause,
      ),
    );
  };

  const draftColumn = draft ? columnsByKey.get(draft.columnKey) : null;
  const draftOperators = draftColumn ? operatorsForColumn(draftColumn) : [];
  const needsValue = draft ? filterNeedsValue(draft.operator) : false;
  const draftCategory = draftColumn ? columnCategory(draftColumn) : null;
  const customValue =
    needsValue && draft && draftColumn && renderFilterValue
      ? renderFilterValue({
          column: draftColumn,
          value: draft.value,
          onChange: (next) => setDraft({ ...draft, value: next }),
          anchorRef: valueAnchorRef,
        })
      : null;
  const temporalControl =
    !customValue && draftColumn && draftCategory === "date"
      ? filterTemporalControl(draftColumn, draft?.value ?? "")
      : null;
  const canApply =
    !!draft &&
    !!draftColumn &&
    (!needsValue ||
      draftCategory === "bool" ||
      draft.value.trim().length > 0);

  // Quick filter targets a single text-ish column (one that accepts `contains`).
  const textColumns = useMemo(
    () => columns.filter((column) => operatorsForColumn(column).includes("contains")),
    [columns],
  );
  const showQuickFilter = !!onQuickFilterChange;
  // Keep the typed text local so each keystroke only re-renders this toolbar,
  // not the whole grid (which can hold 100k+ rows). The trimmed value is pushed
  // to the parent — and the server query — only after the user pauses.
  const [quickDraft, setQuickDraft] = useState(quickFilter ?? "");
  // Adopt external changes (tab switch, programmatic clear) but ignore the
  // parent echoing back our own trimmed value, so a trailing space isn't eaten.
  useEffect(() => {
    setQuickDraft((prev) => (prev.trim() === (quickFilter ?? "") ? prev : quickFilter ?? ""));
  }, [quickFilter]);
  const onQuickRef = useRef(onQuickFilterChange);
  onQuickRef.current = onQuickFilterChange;
  useEffect(() => {
    const handle = setTimeout(() => onQuickRef.current?.(quickDraft.trim()), 250);
    return () => clearTimeout(handle);
  }, [quickDraft]);
  // Saved presets: the "presets" word is the dropdown (apply / hover-× delete);
  // the save button swaps it for a small name input. No selected-preset state —
  // applying is a one-shot action, not a mode.
  const [presetDraft, setPresetDraft] = useState<string | null>(null);
  const presetInputRef = useRef<HTMLInputElement | null>(null);
  useEffect(() => {
    if (presetDraft !== null) presetInputRef.current?.focus();
  }, [presetDraft]);
  const applyPresetDraft = () => {
    const name = presetDraft?.trim();
    if (!name) return;
    savedFilters?.onSave(name);
    setPresetDraft(null);
  };

  const quickValue = quickDraft;
  const quickColumnValue = quickFilterColumn ?? textColumns[0]?.key ?? "";
  const quickActive = quickValue.trim().length > 0;
  const activeCount = filters.length + (quickActive ? 1 : 0);

  return (
    <div className="grid-filterbar">
      {showQuickFilter && (
        <div className="grid-quickfilter">
          <GridIcon.search size={11} style={{ color: "var(--fg-3)" }} />
          <input
            className="grid-quickfilter-input mono"
            value={quickValue}
            onChange={(e) => setQuickDraft(e.target.value)}
            placeholder="Quick filter (id or text)…"
            aria-label="Quick filter"
          />
          {textColumns.length > 0 && (
            <GridSelect
              className="grid-quickfilter-column"
              value={quickColumnValue}
              options={textColumns.map((column) => ({
                value: column.key,
                label: column.name,
              }))}
              onChange={(next) => onQuickFilterColumnChange?.(next || null)}
              aria-label="Quick filter column"
              title="Text column searched by the quick filter (numeric values match the id/primary key)"
            />
          )}
          {quickActive && (
            <button
              className="grid-filter-remove"
              onClick={() => {
                setQuickDraft("");
                onQuickFilterChange?.("");
              }}
              aria-label="Clear quick filter"
            >
              <GridIcon.close size={9} />
            </button>
          )}
        </div>
      )}
      <div className="grid-filterbar-label">
        <GridIcon.filter size={11} style={{ color: "var(--accent)" }} />
        <span>where</span>
        {activeCount > 0 && (
          <span className="grid-filter-active-count" title="Active filters (quick + advanced)">
            {activeCount} active
          </span>
        )}
      </div>
      <div className="grid-filterbar-chips">
        {filters.map((clause, index) => {
          const column = columnsByKey.get(clause.columnKey);
          return (
            <span key={clause.id} className="grid-filter-chip-wrap">
              {index > 0 && (
                <button
                  className="grid-filter-logic"
                  onClick={() => toggleLogic(clause.id)}
                  title="Toggle AND/OR"
                >
                  {clause.logic}
                </button>
              )}
              <span className="grid-filter-chip">
                <button
                  className="grid-filter-chip-main"
                  onClick={() => openEdit(clause)}
                  title="Edit filter"
                >
                  <span className="mono">{column?.name ?? clause.columnKey}</span>
                  <span className="grid-filter-op">
                    {filterOperatorLabel(clause.operator)}
                  </span>
                  {filterNeedsValue(clause.operator) && (
                    <span className="mono grid-filter-value">
                      {filterValuePreview(clause)}
                    </span>
                  )}
                </button>
                <button
                  className="grid-filter-remove"
                  onClick={() => removeFilter(clause.id)}
                  aria-label={`Remove filter on ${column?.name ?? clause.columnKey}`}
                >
                  <GridIcon.close size={9} />
                </button>
              </span>
            </span>
          );
        })}

        {draft ? (
          <div
            className="grid-filter-composer"
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.stopPropagation();
                setDraft(null);
              }
              if (e.key === "Enter") {
                e.preventDefault();
                applyDraft();
              }
            }}
          >
            {filters.length > 0 && !draft.id && (
              <GridSelect
                className="grid-filter-logic-select"
                value={draft.logic}
                options={[
                  { value: "and", label: "and" },
                  { value: "or", label: "or" },
                ]}
                onChange={(next) =>
                  setDraft({ ...draft, logic: next as FilterLogic })
                }
                aria-label="Filter join"
              />
            )}
            <GridSelect
              ref={columnRef}
              value={draft.columnKey}
              options={columns.map((column) => ({
                value: column.key,
                label: column.name,
              }))}
              onChange={(next) => {
                const nextColumn = columnsByKey.get(next);
                if (!nextColumn) return;
                const operator = nextOperatorForColumn(nextColumn, draft.operator);
                setDraft({
                  ...draft,
                  columnKey: nextColumn.key,
                  operator,
                  // Column type drives the value control — reset so stale text
                  // doesn't linger in a bool/date picker.
                  value: defaultFilterValue(nextColumn),
                });
              }}
              aria-label="Filter column"
            />
            <GridSelect
              className="grid-filter-operator-select"
              value={draft.operator}
              options={draftOperators.map((operator) => ({
                value: operator,
                label: filterOperatorLabel(operator),
              }))}
              onChange={(next) =>
                setDraft({
                  ...draft,
                  operator: next as FilterOperator,
                  value: filterNeedsValue(next as FilterOperator)
                    ? draft.value
                    : "",
                })
              }
              aria-label="Filter operator"
            />
            {needsValue && draftCategory === "bool" && (
              <GridSelect
                ref={valueSelectRef}
                className="grid-filter-bool-select"
                value={draft.value === "false" ? "false" : "true"}
                options={BOOL_FILTER_OPTIONS}
                onChange={(next) => setDraft({ ...draft, value: next })}
                aria-label="Filter value"
              />
            )}
            {needsValue && draftCategory !== "bool" && customValue}
            {needsValue && draftCategory !== "bool" && !customValue && temporalControl && (
              <input
                ref={valueRef}
                className={`grid-filter-input mono grid-filter-input-${temporalControl.type}`}
                type={temporalControl.type}
                step={temporalControl.step}
                value={temporalControl.value}
                onChange={(e) => setDraft({ ...draft, value: e.target.value })}
                aria-label="Filter value"
              />
            )}
            {needsValue &&
              draftCategory !== "bool" &&
              !customValue &&
              !temporalControl && (
              <input
                ref={valueRef}
                className="grid-filter-input mono"
                value={draft.value}
                list={draftColumn?.enum ? `filter-values-${draftColumn.key}` : undefined}
                onChange={(e) => setDraft({ ...draft, value: e.target.value })}
                placeholder="value"
                aria-label="Filter value"
              />
            )}
            {draftColumn?.enum && (
              <datalist id={`filter-values-${draftColumn.key}`}>
                {draftColumn.enum.map((value) => (
                  <option key={value} value={value} />
                ))}
              </datalist>
            )}
            <button
              className="grid-filter-apply"
              onClick={applyDraft}
              disabled={!canApply}
            >
              {draft.id ? "save" : "apply"}
            </button>
            <button
              className="grid-filter-cancel"
              onClick={() => setDraft(null)}
              aria-label="Cancel filter"
            >
              <GridIcon.close size={9} />
            </button>
          </div>
        ) : (
          <button className="grid-filter-add" onClick={openAdd} disabled={columns.length === 0}>
            <GridIcon.plus size={10} />
            <span>add</span>
          </button>
        )}

        {filters.length > 1 && (
          <button
            className="grid-filter-clear"
            onClick={() => {
              setFilters([]);
              setDraft(null);
            }}
          >
            clear
          </button>
        )}
      </div>
      {onSortChange && (
        <div className="grid-orderby">
          <span className="grid-filterbar-label">
            {sort?.direction === "desc" ? (
              <GridIcon.sortDesc size={11} style={{ color: "var(--accent)" }} />
            ) : (
              <GridIcon.sortAsc
                size={11}
                style={{ color: sort ? "var(--accent)" : "var(--fg-3)" }}
              />
            )}
            <span>order by</span>
          </span>
          <select
            className="grid-filter-select"
            value={sort?.columnKey ?? ""}
            onChange={(e) =>
              onSortChange(
                e.target.value
                  ? { columnKey: e.target.value, direction: sort?.direction ?? "asc" }
                  : null,
              )
            }
            aria-label="Order by column"
          >
            <option value="">—</option>
            {columns.map((column) => (
              <option key={column.key} value={column.key}>
                {column.name}
              </option>
            ))}
          </select>
          {sort && (
            <button
              className="grid-filter-logic"
              onClick={() =>
                onSortChange({
                  columnKey: sort.columnKey,
                  direction: sort.direction === "asc" ? "desc" : "asc",
                })
              }
              title="Toggle sort direction"
            >
              {sort.direction}
            </button>
          )}
        </div>
      )}
      {savedFilters && (
        <div className="grid-orderby">
          {presetDraft !== null ? (
            <>
              <input
                ref={presetInputRef}
                className="grid-filter-input mono"
                value={presetDraft}
                onChange={(e) => setPresetDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    applyPresetDraft();
                  }
                  if (e.key === "Escape") {
                    e.stopPropagation();
                    setPresetDraft(null);
                  }
                }}
                placeholder="preset name"
                aria-label="Preset name"
              />
              <button
                className="grid-filter-apply"
                onClick={applyPresetDraft}
                disabled={!presetDraft.trim()}
              >
                save
              </button>
              <button
                className="grid-filter-cancel"
                onClick={() => setPresetDraft(null)}
                aria-label="Cancel saving preset"
              >
                <GridIcon.close size={9} />
              </button>
            </>
          ) : (
            <PresetMenu
              names={savedFilters.names}
              activeName={savedFilters.activeName ?? null}
              onApply={savedFilters.onApply}
              onDelete={savedFilters.onDelete}
              onClear={savedFilters.onClear}
              onSaveRequest={() => setPresetDraft("")}
            />
          )}
        </div>
      )}
      <div
        className="grid-filterbar-summary mono"
        title={
          serverRows === undefined
            ? "Filtered rows in the loaded page"
            : `Filtered rows in the loaded page. Server total: ${serverRows.toLocaleString()}`
        }
      >
        <span style={{ color: "var(--fg-1)" }}>
          {filteredRows.toLocaleString()}
        </span>
        <span style={{ color: "var(--fg-3)" }}>/</span>
        <span style={{ color: "var(--fg-2)" }}>{totalRows.toLocaleString()}</span>
        <span style={{ color: "var(--fg-3)" }}> page rows</span>
      </div>
    </div>
  );
}
