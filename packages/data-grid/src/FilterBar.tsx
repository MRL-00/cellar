import { useEffect, useMemo, useRef, useState } from "react";
import {
  createFilterId,
  filterNeedsValue,
  filterOperatorLabel,
  filterValuePreview,
  nextOperatorForColumn,
  operatorsForColumn,
} from "./filters";
import { GridIcon } from "./icons";
import type {
  ColumnFilters,
  FilterClause,
  FilterLogic,
  FilterOperator,
  GridColumn,
} from "./types";

type ComposerDraft = {
  id: string | null;
  columnKey: string;
  operator: FilterOperator;
  value: string;
  logic: FilterLogic;
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
};

export function FilterBar({
  columns,
  filters,
  setFilters,
  quickFilter,
  onQuickFilterChange,
  quickFilterColumn,
  onQuickFilterColumnChange,
}: FilterBarProps) {
  const [draft, setDraft] = useState<ComposerDraft | null>(null);
  const columnRef = useRef<HTMLSelectElement | null>(null);
  const valueRef = useRef<HTMLInputElement | null>(null);
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
      valueRef.current?.focus();
      valueRef.current?.select();
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
      value: "",
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
    const value = draft.value.trim();
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
  const canApply =
    !!draft && !!draftColumn && (!needsValue || draft.value.trim().length > 0);

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
            <select
              className="grid-filter-select grid-quickfilter-column"
              value={quickColumnValue}
              onChange={(e) => onQuickFilterColumnChange?.(e.target.value || null)}
              aria-label="Quick filter column"
              title="Text column searched by the quick filter (numeric values match the id/primary key)"
            >
              {textColumns.map((column) => (
                <option key={column.key} value={column.key}>
                  {column.name}
                </option>
              ))}
            </select>
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
              <select
                className="grid-filter-select grid-filter-logic-select"
                value={draft.logic}
                onChange={(e) =>
                  setDraft({ ...draft, logic: e.target.value as FilterLogic })
                }
                aria-label="Filter join"
              >
                <option value="and">and</option>
                <option value="or">or</option>
              </select>
            )}
            <select
              ref={columnRef}
              className="grid-filter-select"
              value={draft.columnKey}
              onChange={(e) => {
                const nextColumn = columnsByKey.get(e.target.value);
                if (!nextColumn) return;
                const operator = nextOperatorForColumn(nextColumn, draft.operator);
                setDraft({ ...draft, columnKey: nextColumn.key, operator });
              }}
              aria-label="Filter column"
            >
              {columns.map((column) => (
                <option key={column.key} value={column.key}>
                  {column.name}
                </option>
              ))}
            </select>
            <select
              className="grid-filter-select grid-filter-operator-select"
              value={draft.operator}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  operator: e.target.value as FilterOperator,
                  value: filterNeedsValue(e.target.value as FilterOperator)
                    ? draft.value
                    : "",
                })
              }
              aria-label="Filter operator"
            >
              {draftOperators.map((operator) => (
                <option key={operator} value={operator}>
                  {filterOperatorLabel(operator)}
                </option>
              ))}
            </select>
            {needsValue && (
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
    </div>
  );
}
