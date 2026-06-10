/**
 * Unit tests for CellEditor event-handler semantics.
 *
 * Tests are written against the exported pure helper functions
 * resolveEnterAction and resolveBlurAction from Cell.tsx.  These functions
 * contain the actual production decision logic, so a regression in Cell.tsx
 * (e.g. wrong dirty/settled logic) will cause these tests to fail.
 */

import { describe, expect, it } from "vitest";
import { resolveBlurAction, resolveEnterAction } from "./Cell";

// ---------------------------------------------------------------------------
// resolveEnterAction — used by the Enter key handler (text editor)
// ---------------------------------------------------------------------------

describe("resolveEnterAction", () => {
  it("commits when the value was modified (dirty=true)", () => {
    expect(resolveEnterAction(true)).toBe("commit");
  });

  it("cancels when the value was never modified (dirty=false)", () => {
    // Covers B2: opening a NULL cell and pressing Enter without typing
    // must not produce a phantom pending edit.
    expect(resolveEnterAction(false)).toBe("cancel");
  });
});

// ---------------------------------------------------------------------------
// resolveBlurAction — used by onBlur in both text and enum editors
// ---------------------------------------------------------------------------

describe("resolveBlurAction", () => {
  // B1: Escape sets settled=true; the subsequent blur must be a no-op.
  it("returns noop when settled=true and dirty=false (B1: Escape-then-blur)", () => {
    expect(resolveBlurAction(true, false)).toBe("noop");
  });

  // B1 + B13: settled=true blocks blur regardless of dirty.
  it("returns noop when settled=true and dirty=true (B13: Enter-then-blur)", () => {
    expect(resolveBlurAction(true, true)).toBe("noop");
  });

  it("commits on blur when the editor is not settled and the value was modified", () => {
    expect(resolveBlurAction(false, true)).toBe("commit");
  });

  // B2: Opening a NULL cell and blurring without typing must not record an edit.
  it("cancels on blur when the editor is not settled and the value was never modified (B2: NULL cell blur)", () => {
    expect(resolveBlurAction(false, false)).toBe("cancel");
  });
});

// ---------------------------------------------------------------------------
// Enum editor path — blur-cancel and Escape semantics
//
// The enum editor's container div uses the same settledRef guard and calls
// resolveBlurAction(settled, false) — dirty is always false for enum cells
// because the user either clicks a button (commit, sets settled) or blurs/
// Escapes without selecting (cancel).
// ---------------------------------------------------------------------------

describe("resolveBlurAction — enum editor path", () => {
  it("cancels on container blur when no option was clicked (settled=false)", () => {
    // Represents clicking outside the enum editor without picking an option.
    expect(resolveBlurAction(false, false)).toBe("cancel");
  });

  it("returns noop on container blur after an option button was already clicked (settled=true)", () => {
    // The button click sets settled=true before onCommit fires; the resulting
    // focus-leave on the container must not trigger a second cancel.
    expect(resolveBlurAction(true, false)).toBe("noop");
  });

  it("returns noop on container blur after Escape was pressed (settled=true)", () => {
    // Escape sets settled=true and calls onCancel; the container blur that
    // follows (focus moves elsewhere) must be ignored.
    expect(resolveBlurAction(true, false)).toBe("noop");
  });
});

// ---------------------------------------------------------------------------
// End-to-end state machine walkthroughs
// ---------------------------------------------------------------------------

describe("CellEditor state machine — text editor", () => {
  it("B1: Escape cancels; subsequent blur is a no-op", () => {
    // Escape sets settled=true and calls onCancel.
    // The blur that fires when the input unmounts must be ignored.
    let settled = false;

    // Escape keydown
    settled = true;
    // (onCancel would be called here in the component)

    expect(resolveBlurAction(settled, false)).toBe("noop");
  });

  it("B13: Enter commits exactly once; subsequent blur is a no-op", () => {
    let settled = false;
    const dirty = true;

    // Enter keydown
    expect(resolveEnterAction(dirty)).toBe("commit");
    settled = true;

    // Subsequent blur from input unmounting
    expect(resolveBlurAction(settled, dirty)).toBe("noop");
  });

  it("B2: NULL cell opened — Enter without typing does not commit", () => {
    // initialValue is null → dirty starts as false
    expect(resolveEnterAction(false)).toBe("cancel");
  });

  it("B2: NULL cell opened — blur without typing does not commit", () => {
    expect(resolveBlurAction(false, false)).toBe("cancel");
  });

  it("typing into a NULL cell and pressing Enter commits exactly once", () => {
    let settled = false;
    const dirty = true; // user typed something

    expect(resolveEnterAction(dirty)).toBe("commit");
    settled = true;

    // Blur from unmounting must be ignored.
    expect(resolveBlurAction(settled, dirty)).toBe("noop");
  });

  it("typing a value and blurring commits once", () => {
    const settled = false;
    const dirty = true;

    expect(resolveBlurAction(settled, dirty)).toBe("commit");
  });
});
