import { memo, useRef } from "react";
import { CellEditor, CellValue, type CellEditorProps } from "./Cell";
import type { CellEditorRenderer, DataGridProps } from "./DataGridProps";
import type { RendererRegistry, SaveBlob } from "./renderers/types";
import type {
  CellAddress,
  CellChange,
  GridColumn,
  GridRow,
  PendingChange,
} from "./types";

export type GridRowViewProps = {
  row: GridRow;
  rowIndex: number;
  rowNumber: number;
  columns: readonly GridColumn[];
  change: PendingChange | undefined;
  selected: CellAddress | null;
  rowSelected: boolean;
  editing: CellAddress | null;
  frozenCount: number;
  readOnly: boolean;
  nullDisplay: string;
  renderers: RendererRegistry | null;
  renderEditor: CellEditorRenderer | undefined;
  saveBlob: SaveBlob | undefined;
  stripeRows: boolean;
  top: number | undefined;
  onSelect: (next: CellAddress | null) => void;
  onEdit: (next: CellAddress | null) => void;
  onCellEdit: (
    rowId: string,
    colKey: string,
    prev: CellChange["from"],
    next: CellChange["to"],
  ) => void;
  onCellContextMenu: DataGridProps["onCellContextMenu"];
  onRowSelect: DataGridProps["onRowSelect"];
  onRowContextMenu: DataGridProps["onRowContextMenu"];
};

export const GridRowView = memo(function GridRowView({
  row,
  rowIndex,
  rowNumber,
  columns,
  change,
  selected,
  rowSelected,
  editing,
  frozenCount,
  readOnly,
  nullDisplay,
  renderers,
  renderEditor,
  saveBlob,
  stripeRows,
  top,
  onSelect,
  onEdit,
  onCellEdit,
  onCellContextMenu,
  onRowSelect,
  onRowContextMenu,
}: GridRowViewProps) {
  const kind = change?.kind;
  // Set when an inline editor commits/cancels. A double-click on an option
  // button (TRUE/FALSE, enum chips) closes the editor on the first click; the
  // second click would land on the cell and instantly reopen it, so cell
  // double-clicks are ignored for a beat after the editor settles.
  const editorSettledAt = useRef(0);
  // A single selected cell gives the row a faint tint; clicking the row-number
  // gutter selects the whole row (a stronger highlight). The two are mutually
  // exclusive — selecting one clears the other in the handlers below.
  const cellInRow = selected !== null;
  const rowGutterInteractive = onRowSelect !== undefined;

  // Compute stripe class from absolute rowIndex so it stays correct in virtual
  // scroll mode (where nth-child reflects only the current render window).
  // Only apply the stripe when there is no pending-change tint (is-update,
  // is-insert, is-delete), so the change indicators are never hidden.
  const isStripe = stripeRows && !kind && rowIndex % 2 === 1;

  return (
    <div
      className={
        "grid-row" +
        (kind ? " is-" + kind : "") +
        (isStripe ? " is-stripe" : "") +
        (cellInRow ? " is-selected-row" : "") +
        (rowSelected ? " is-row-selected" : "")
      }
      style={top === undefined ? undefined : { top }}
    >
      <div
        className={
          "grid-cell grid-cell-rowno" +
          (rowGutterInteractive ? " is-interactive" : "") +
          (rowSelected ? " is-active" : "")
        }
        role={rowGutterInteractive ? "button" : undefined}
        tabIndex={rowGutterInteractive ? 0 : undefined}
        aria-pressed={rowGutterInteractive ? rowSelected : undefined}
        title={rowGutterInteractive ? "Select row" : undefined}
        onClick={
          rowGutterInteractive
            ? () => {
                onSelect(null);
                onEdit(null);
                onRowSelect?.(
                  rowSelected ? null : rowIndex,
                  rowSelected ? null : row,
                );
              }
            : undefined
        }
        onKeyDown={
          rowGutterInteractive
            ? (event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onSelect(null);
                  onEdit(null);
                  onRowSelect?.(
                    rowSelected ? null : rowIndex,
                    rowSelected ? null : row,
                  );
                }
              }
            : undefined
        }
        onContextMenu={
          onRowContextMenu &&
          ((event) => {
            onSelect(null);
            onEdit(null);
            onRowSelect?.(rowIndex, row);
            onRowContextMenu(event, row, rowIndex);
          })
        }
      >
        <span className="grid-rowno-num tnum">{rowNumber}</span>
        {kind === "update" && (
          <span
            className="grid-gutter-mark"
            style={{ background: "var(--update)" }}
            title="Updated"
          />
        )}
        {kind === "insert" && (
          <span
            className="grid-gutter-mark"
            style={{ background: "var(--insert)" }}
            title="Inserted"
          />
        )}
        {kind === "delete" && (
          <span
            className="grid-gutter-mark"
            style={{ background: "var(--delete)" }}
            title="Marked for delete"
          />
        )}
      </div>
      {columns.map((c, ci) => {
        const isSel = selected?.col === ci;
        const isEdit = !readOnly && editing?.col === ci;
        const cellChange = change?.edits?.[c.key];
        const displayed = cellChange ? cellChange.to : row[c.key];
        const original = row[c.key] ?? null;

        return (
          <div
            key={c.key}
            className={
              "grid-cell" +
              (ci < frozenCount ? " frozen" : "") +
              (cellChange ? " is-edited" : "") +
              (isSel ? " is-selected" : "") +
              (isEdit ? " is-editing" : "")
            }
            style={{ width: c.width, flexBasis: c.width }}
            onClick={() => {
              if (rowSelected) onRowSelect?.(null, null);
              onSelect({ row: rowIndex, col: ci });
            }}
            onDoubleClick={() => {
              if (readOnly) return;
              if (Date.now() - editorSettledAt.current < 350) return;
              onEdit({ row: rowIndex, col: ci });
            }}
            onContextMenu={
              onCellContextMenu &&
              ((event) => {
                if (rowSelected) onRowSelect?.(null, null);
                onSelect({ row: rowIndex, col: ci });
                onCellContextMenu(event, row, c);
              })
            }
          >
            {isEdit ? (
              (() => {
                const editorProps: CellEditorProps = {
                  col: c,
                  value: displayed,
                  onCommit: (v) => {
                    editorSettledAt.current = Date.now();
                    onCellEdit(
                      row.id,
                      c.key,
                      (original ?? null) as CellChange["from"],
                      (v ?? null) as CellChange["to"],
                    );
                    onEdit(null);
                  },
                  onCancel: () => {
                    editorSettledAt.current = Date.now();
                    onEdit(null);
                  },
                };
                // Host-supplied editor wins when it claims the cell; otherwise
                // fall back to the built-in text/number/native-picker editor.
                return renderEditor?.(editorProps) ?? <CellEditor {...editorProps} />;
              })()
            ) : (
              <CellValue
                col={c}
                value={displayed}
                nullDisplay={nullDisplay}
                renderers={renderers}
                saveBlob={saveBlob}
              />
            )}
            {cellChange && !isEdit && (
              <span
                className="grid-cell-prev"
                title={`Was: ${cellChange.from ?? "NULL"}`}
              >
                <span className="grid-cell-prev-strike">
                  {cellChange.from === null ? "NULL" : String(cellChange.from)}
                </span>
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
});
