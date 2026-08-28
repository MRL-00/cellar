# GPUI grid prototype verdict

Question: can GPUI render Cellar-scale database grids smoothly enough to justify a native migration?

Verdict: yes, for the grid rendering path. On 14 August 2026, all three workloads felt smooth after virtualizing both rows and columns:

- 500 rows × 50 columns
- 500 rows × 500 columns
- 10,000 rows × 500 columns

The first implementation virtualized rows only; the two 500-column workloads were very laggy. GPUI alone did not fix an unbounded render tree. Once horizontal virtualization bounded the rendered cells to the viewport, all workloads felt good.

This validates a GPUI vertical slice, not an immediate whole-app rewrite. The next proof should reuse Cellar's existing Rust core and add real streamed query pages, selection, editing, frozen columns, and keyboard navigation. Delete this prototype after that production slice absorbs the result.
