/**
 * Unit tests for CellEditor event-handler semantics.
 *
 * Tests are written against the exported pure helper functions
 * resolveEnterAction and resolveBlurAction from Cell.tsx.  These functions
 * contain the actual production decision logic, so a regression in Cell.tsx
 * (e.g. wrong dirty/settled logic) will cause these tests to fail.
 */

import { describe, expect, it } from "vitest";
import {
  nativeControl,
  parseCellInput,
  resolveBlurAction,
  resolveEnterAction,
} from "./Cell";

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

describe("parseCellInput", () => {
  it("coerces boolean values", () => {
    expect(parseCellInput(column("active", "bool"), "true")).toEqual({
      ok: true,
      value: true,
    });
    expect(parseCellInput(column("active", "bool"), "0")).toEqual({
      ok: true,
      value: false,
    });
  });

  it("rejects non-boolean values for boolean columns", () => {
    expect(parseCellInput(column("active", "bool"), "sometimes")).toEqual({
      ok: false,
      message: "Enter TRUE or FALSE",
    });
  });

  it("coerces integer values and rejects fractional input", () => {
    expect(parseCellInput(column("count", "int4"), "42")).toEqual({
      ok: true,
      value: 42,
    });
    expect(parseCellInput(column("count", "int4"), "4.2")).toEqual({
      ok: false,
      message: "Enter a whole number",
    });
  });

  it("keeps large integer values as strings to avoid precision loss", () => {
    expect(parseCellInput(column("id", "int8"), "9007199254740993")).toEqual({
      ok: true,
      value: "9007199254740993",
    });
  });

  it("keeps numeric decimals as strings to avoid precision loss", () => {
    expect(parseCellInput(column("amount", "numeric"), "123.45")).toEqual({
      ok: true,
      value: "123.45",
    });
  });

  it("validates JSON and GUID fields", () => {
    expect(parseCellInput(column("payload", "jsonb"), '{"ok":true}')).toEqual({
      ok: true,
      value: '{"ok":true}',
    });
    expect(parseCellInput(column("payload", "jsonb"), "{oops")).toEqual({
      ok: false,
      message: "Enter valid JSON",
    });
    expect(
      parseCellInput(
        column("id", "uuid"),
        "018f61b3-4f51-7f5a-9db8-0c7b1b7b3930",
      ),
    ).toEqual({
      ok: true,
      value: "018f61b3-4f51-7f5a-9db8-0c7b1b7b3930",
    });
    expect(
      parseCellInput(
        column("id", "uuid"),
        "018f61b3-4f51-7f5a-9db8",
      ),
    ).toEqual({
      ok: false,
      message: "Enter a valid GUID",
    });
    expect(parseCellInput(column("Id", "guid"), "aaa")).toEqual({
      ok: false,
      message: "Enter a valid GUID",
    });
    expect(parseCellInput(column("Id", "uniqueidentifier"), "aaa")).toEqual({
      ok: false,
      message: "Enter a valid GUID",
    });
  });

  it("validates extended blob types as hex, matching the bytea renderer", () => {
    expect(parseCellInput(column("thumb", "longblob"), "\\xdeadbeef")).toEqual({
      ok: true,
      value: "\\xdeadbeef",
    });
    expect(parseCellInput(column("thumb", "image"), "not hex")).toEqual({
      ok: false,
      message: "Use hex bytea format, e.g. \\x0a2b",
    });
  });

  it("maps blank nullable non-text values to NULL", () => {
    expect(parseCellInput(column("due_at", "timestamptz", true), "")).toEqual({
      ok: true,
      value: null,
    });
  });

  it("rejects blank non-null non-text values", () => {
    expect(parseCellInput(column("due_at", "timestamptz", false), "")).toEqual({
      ok: false,
      message: "due_at cannot be NULL",
    });
  });

  it("allows empty strings for text columns", () => {
    expect(parseCellInput(column("name", "text", false), "")).toEqual({
      ok: true,
      value: "",
    });
  });

  it("validates MSSQL/MySQL type aliases instead of accepting any text", () => {
    // The bug: `datetime2` fell through to "unknown" and accepted "aaa".
    expect(parseCellInput(column("CreationTime", "datetime2(7)"), "aaa")).toEqual({
      ok: false,
      message: "Enter a valid timestamp",
    });
    expect(
      parseCellInput(column("CreationTime", "datetime2(7)"), "2023-04-03T05:00:31"),
    ).toEqual({ ok: true, value: "2023-04-03T05:00:31" });
    expect(parseCellInput(column("count", "int"), "abc")).toEqual({
      ok: false,
      message: "Enter a whole number",
    });
    expect(parseCellInput(column("flag", "tinyint"), "3")).toEqual({
      ok: true,
      value: 3,
    });
    expect(parseCellInput(column("note", "nvarchar(100)"), "hello")).toEqual({
      ok: true,
      value: "hello",
    });
  });

  it("requires enum values to match the column options", () => {
    expect(
      parseCellInput(
        { ...column("status", "text"), enum: ["new", "done"] },
        "done",
      ),
    ).toEqual({ ok: true, value: "done" });
    expect(
      parseCellInput(
        { ...column("status", "text"), enum: ["new", "done"] },
        "stuck",
      ),
    ).toEqual({
      ok: false,
      message: "Choose one of: new, done",
    });
  });
});

describe("nativeControl", () => {
  it("offers a datetime-local picker for timestamp columns, dropping sub-second precision", () => {
    expect(
      nativeControl(column("CreationTime", "datetime2(7)"), "2023-04-03T05:00:31.15863"),
    ).toEqual({ type: "datetime-local", step: "1", value: "2023-04-03T05:00:31" });
  });

  it("offers a date picker for date columns", () => {
    expect(nativeControl(column("d", "date"), "2023-04-03")).toEqual({
      type: "date",
      value: "2023-04-03",
    });
  });

  it("offers a number input for integer and float columns", () => {
    expect(nativeControl(column("n", "int"), "42")).toEqual({
      type: "number",
      step: "1",
      value: "42",
    });
    expect(nativeControl(column("f", "float8"), "4.2")).toEqual({
      type: "number",
      step: "any",
      value: "4.2",
    });
  });

  it("still offers a picker for an empty/NULL date cell", () => {
    expect(nativeControl(column("d", "datetime2"), "")).toEqual({
      type: "datetime-local",
      step: "1",
      value: "",
    });
  });

  it("falls back to text for non-native and unparseable values", () => {
    expect(nativeControl(column("note", "nvarchar(100)"), "hi")).toBeNull();
    // A legacy/garbage timestamp that can't be coerced stays editable as text.
    expect(nativeControl(column("CreationTime", "datetime2"), "aaa")).toBeNull();
  });
});

function column(name: string, type: string, nullable = true) {
  return {
    key: name,
    name,
    type,
    width: 120,
    nullable,
  };
}
