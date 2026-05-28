import { useState } from "react";
import { DataGrid, useGridState } from "@cellar/data-grid";
import {
  ORDER_COLUMNS,
  ORDERS_TOTAL_ROWS,
  SAMPLE_ORDERS,
  makeSamplePendingChanges,
} from "../lib/sampleOrders";

export function Workspace({ onCommit }: { onCommit?: () => void } = {}) {
  // Stash the seeded pending changes in lazy-init state so React re-renders
  // (theme toggles, settings panel) don't reset what the user has edited.
  const [seededChanges] = useState(makeSamplePendingChanges);
  const grid = useGridState({ initialChanges: seededChanges });

  return (
    <div className="flex flex-1 min-h-0 overflow-hidden">
      <DataGrid
        columns={ORDER_COLUMNS}
        rows={SAMPLE_ORDERS}
        totalRows={ORDERS_TOTAL_ROWS}
        changes={grid.changes}
        onChange={grid.setChanges}
        selection={grid.selection}
        onSelect={grid.setSelection}
        editing={grid.editing}
        onEdit={grid.setEditing}
        filters={grid.filters}
        onFiltersChange={grid.setFilters}
        onCommit={onCommit}
        onRevert={grid.revert}
      />
    </div>
  );
}
