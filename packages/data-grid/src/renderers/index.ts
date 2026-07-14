/** Public surface of the pluggable cell-renderer system. */
export type {
  CellRenderer,
  CellRenderContext,
  CellRenderMode,
  NonNullGridValue,
  RendererRegistry,
  SaveBlob,
} from "./types";
export {
  builtinRenderers,
  createRendererRegistry,
  defaultRendererRegistry,
  extendRegistry,
} from "./registry";
export { RichCell, type RichCellProps } from "./RichCell";
export { Popover, type PopoverProps } from "./Popover";
export { defaultSaveBlob } from "./shared";

// Built-in renderers + their pure helpers (handy for tests and custom registries).
export { jsonRenderer, safeParseJson, jsonKind, jsonSummary } from "./json";
export {
  arrayRenderer,
  parsePgArray,
  elementOf,
  type ArrayElement,
  type ArrayElementKind,
} from "./array";
export {
  byteaRenderer,
  parseHexBytes,
  byteaInfo,
  sniffImageMime,
  formatByteSize,
  hexDump,
  type ByteaInfo,
  type HexDumpRow,
} from "./bytes";
export { geometryRenderer, geometryLabel } from "./geometry";
export {
  baseType,
  isJsonType,
  isGuidType,
  isArrayType,
  arrayElementType,
  isByteaType,
  isGeometryType,
} from "./typeMatch";
