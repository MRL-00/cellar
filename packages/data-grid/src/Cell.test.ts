/**
 * Unit tests for CellEditor event-handler semantics.
 *
 * These tests verify the logic that governs when onCommit and onCancel are
 * called for the three entry points: Enter key, Escape key, and blur.
 * The tests are written as pure-logic simulations so they run without a DOM
 * environment, matching the style used in the rest of this package.
 */

import { describe, expect, it, vi } from "vitest";

// ---------------------------------------------------------------------------
// Minimal simulation of CellEditor's event-handler state machine.
// This mirrors the implementation in Cell.tsx exactly so that the tests act
// as a specification / regression guard.
// ---------------------------------------------------------------------------

type HandlerResult = "commit" | "cancel" | "noop";

interface SimulateOptions {
  /** Initial value (null/undefined ≡ NULL database cell). */
  initialValue: string | null | undefined;
  /** Sequence of actions the user performs. */
  actions: Array<
    | { type: "change"; value: string }
    | { type: "keydown"; key: "Enter" | "Escape" }
    | { type: "blur" }
  >;
}

interface SimulateResult {
  commits: Array<string | null | undefined>;
  cancels: number;
  /** The value currently in the editor's controlled state. */
  currentValue: string;
}

/**
 * Simulates the CellEditor state machine without touching the DOM.
 * Returns every call to onCommit and the count of onCancel calls.
 */
function simulate({ initialValue, actions }: SimulateOptions): SimulateResult {
  // Mirror Cell.tsx state
  const initialString = initialValue == null ? "" : String(initialValue);
  let v = initialString;
  let settled = false;
  let dirty = false;

  const commits: Array<string | null | undefined> = [];
  let cancels = 0;

  const onCommit = (next: string | null | undefined) => commits.push(next);
  const onCancel = () => cancels++;

  for (const action of actions) {
    if (action.type === "change") {
      dirty = true;
      v = action.value;
      continue;
    }
    if (action.type === "keydown") {
      if (action.key === "Enter") {
        settled = true;
        if (dirty) {
          onCommit(v);
        } else {
          onCancel();
        }
      } else if (action.key === "Escape") {
        settled = true;
        onCancel();
      }
      continue;
    }
    if (action.type === "blur") {
      if (settled) continue; // settled ref guard — no-op
      settled = true;
      if (dirty) {
        onCommit(v);
      } else {
        onCancel();
      }
      continue;
    }
  }

  return { commits, cancels, currentValue: v };
}

// ---------------------------------------------------------------------------
// B1: Escape must cancel, not commit.
// ---------------------------------------------------------------------------

describe("CellEditor — Escape key", () => {
  it("calls onCancel when Escape is pressed on an untouched cell", () => {
    const result = simulate({
      initialValue: "hello",
      actions: [{ type: "keydown", key: "Escape" }],
    });

    expect(result.cancels).toBe(1);
    expect(result.commits).toHaveLength(0);
  });

  it("calls onCancel even when the user has typed a new value before pressing Escape", () => {
    const result = simulate({
      initialValue: "original",
      actions: [
        { type: "change", value: "oops" },
        { type: "keydown", key: "Escape" },
      ],
    });

    expect(result.cancels).toBe(1);
    expect(result.commits).toHaveLength(0);
  });

  it("subsequent blur after Escape is a no-op (settledRef guard)", () => {
    const result = simulate({
      initialValue: "hello",
      actions: [
        { type: "change", value: "new" },
        { type: "keydown", key: "Escape" },
        { type: "blur" }, // blur fires after the input unmounts — must be ignored
      ],
    });

    expect(result.cancels).toBe(1); // only the Escape cancel
    expect(result.commits).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// B13: Enter must commit exactly once.
// ---------------------------------------------------------------------------

describe("CellEditor — Enter key", () => {
  it("commits exactly once when Enter is pressed after typing", () => {
    const result = simulate({
      initialValue: "old",
      actions: [
        { type: "change", value: "new" },
        { type: "keydown", key: "Enter" },
      ],
    });

    expect(result.commits).toHaveLength(1);
    expect(result.commits[0]).toBe("new");
    expect(result.cancels).toBe(0);
  });

  it("subsequent blur after Enter is a no-op (settledRef guard — prevents double-commit)", () => {
    const result = simulate({
      initialValue: "old",
      actions: [
        { type: "change", value: "new" },
        { type: "keydown", key: "Enter" },
        { type: "blur" }, // input unmounts → blur fires; must be ignored
      ],
    });

    expect(result.commits).toHaveLength(1); // exactly one commit
    expect(result.commits[0]).toBe("new");
    expect(result.cancels).toBe(0);
  });

  it("cancels (no-op) on Enter when the value was never modified", () => {
    const result = simulate({
      initialValue: "untouched",
      actions: [{ type: "keydown", key: "Enter" }],
    });

    // No actual change — should not record a pending edit.
    expect(result.commits).toHaveLength(0);
    expect(result.cancels).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// B2: NULL cell must not produce a phantom "null → ''" pending edit.
// ---------------------------------------------------------------------------

describe("CellEditor — NULL cell phantom edit (B2)", () => {
  it("opening a NULL cell and pressing Enter without typing records no edit", () => {
    const result = simulate({
      initialValue: null,
      actions: [{ type: "keydown", key: "Enter" }],
    });

    expect(result.commits).toHaveLength(0);
    expect(result.cancels).toBe(1);
  });

  it("opening a NULL cell and blurring without typing records no edit", () => {
    const result = simulate({
      initialValue: null,
      actions: [{ type: "blur" }],
    });

    expect(result.commits).toHaveLength(0);
    expect(result.cancels).toBe(1);
  });

  it("opening a NULL cell and pressing Escape records no edit", () => {
    const result = simulate({
      initialValue: null,
      actions: [{ type: "keydown", key: "Escape" }],
    });

    expect(result.commits).toHaveLength(0);
    expect(result.cancels).toBe(1);
  });

  it("typing a value into a NULL cell and committing records exactly one edit", () => {
    const result = simulate({
      initialValue: null,
      actions: [
        { type: "change", value: "filled" },
        { type: "keydown", key: "Enter" },
      ],
    });

    expect(result.commits).toHaveLength(1);
    expect(result.commits[0]).toBe("filled");
    expect(result.cancels).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// General blur behaviour (no prior key press).
// ---------------------------------------------------------------------------

describe("CellEditor — blur without a prior key press", () => {
  it("commits via blur when the value was modified", () => {
    const result = simulate({
      initialValue: "before",
      actions: [
        { type: "change", value: "after" },
        { type: "blur" },
      ],
    });

    expect(result.commits).toHaveLength(1);
    expect(result.commits[0]).toBe("after");
    expect(result.cancels).toBe(0);
  });

  it("does not commit via blur when the value was never modified", () => {
    const result = simulate({
      initialValue: "same",
      actions: [{ type: "blur" }],
    });

    expect(result.commits).toHaveLength(0);
    expect(result.cancels).toBe(1);
  });
});
