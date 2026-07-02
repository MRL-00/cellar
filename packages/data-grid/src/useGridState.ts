import { useCallback, useState } from "react";
import type {
  CellAddress,
  ColumnFilters,
  PendingChanges,
  SortState,
} from "./types";

export type UseGridStateOptions = {
  initialFilters?: ColumnFilters;
  initialChanges?: PendingChanges;
  initialSort?: SortState;
};

/**
 * Minimal local controller for the grid. Suits tabs that don't have a backing
 * Zustand store yet; once we wire `useTabs`, the same pieces can be lifted into
 * a store and the grid stays exactly the same.
 */
export function useGridState({
  initialFilters = [],
  initialChanges = {},
  initialSort = null,
}: UseGridStateOptions = {}) {
  const [filters, setFilters] = useState<ColumnFilters>(initialFilters);
  const [changes, setChanges] = useState<PendingChanges>(initialChanges);
  const [sort, setSort] = useState<SortState>(initialSort);
  const [selection, setSelection] = useState<CellAddress | null>(null);
  const [editing, setEditing] = useState<CellAddress | null>(null);
  const [selectedRow, setSelectedRow] = useState<number | null>(null);

  const revert = useCallback(() => setChanges({}), []);

  return {
    filters,
    setFilters,
    changes,
    setChanges,
    sort,
    setSort,
    selection,
    setSelection,
    editing,
    setEditing,
    selectedRow,
    setSelectedRow,
    revert,
  };
}
