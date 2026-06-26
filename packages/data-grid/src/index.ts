export type {
  CellAddress,
  CellAlign,
  CellChange,
  ChangeKind,
  ColumnFilters,
  FilterClause,
  FilterLogic,
  FilterOperator,
  GridColumn,
  GridColumnLayout,
  GridRow,
  GridValue,
  GridPagination,
  GridStatusCounts,
  PendingChange,
  PendingChanges,
  SortClause,
  SortDirection,
  SortState,
} from "./types";

export { countChanges, statusDotColor, statusTextColor } from "./status";
export {
  FILTER_OPERATORS,
  createFilterId,
  evaluateFilterClause,
  filterNeedsValue,
  filterOperatorLabel,
  filterRows,
  filterValuePreview,
  nextOperatorForColumn,
  operatorsForColumn,
  rowMatchesFilters,
} from "./filters";
export { compareGridValues, cycleSortState, sortGridRows } from "./sort";
export { DataGrid, useGridState, type DataGridProps } from "./DataGrid";
export { CellEditor, CellValue, type CellEditorProps } from "./Cell";
export { FilterBar, type FilterBarProps } from "./FilterBar";
export { PendingBar, type PendingBarProps } from "./PendingBar";
export { TypeIcon } from "./TypeIcon";

// Pluggable rich-type cell renderers (SPEC §9 — renderers are pluggable).
export {
  type CellRenderer,
  type CellRenderContext,
  type CellRenderMode,
  type NonNullGridValue,
  type RendererRegistry,
  type SaveBlob,
  type RichCellProps,
  type PopoverProps,
  type ArrayElement,
  type ArrayElementKind,
  type HexDumpRow,
  builtinRenderers,
  createRendererRegistry,
  defaultRendererRegistry,
  extendRegistry,
  RichCell,
  Popover,
  defaultSaveBlob,
  jsonRenderer,
  safeParseJson,
  jsonKind,
  jsonSummary,
  arrayRenderer,
  parsePgArray,
  elementOf,
  byteaRenderer,
  parseHexBytes,
  sniffImageMime,
  formatByteSize,
  hexDump,
  geometryRenderer,
  geometryLabel,
  baseType,
  isJsonType,
  isArrayType,
  arrayElementType,
  isByteaType,
  isGeometryType,
} from "./renderers";
