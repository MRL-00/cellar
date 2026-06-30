import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULTS, applySettingsSideEffects } from "./settings";

describe("applySettingsSideEffects", () => {
  let originalDocument: typeof globalThis.document;
  let originalWindow: typeof globalThis.window;

  beforeEach(() => {
    originalDocument = globalThis.document;
    originalWindow = globalThis.window;

    const documentElement = fakeElement();
    const body = fakeElement();
    Object.defineProperty(globalThis, "document", {
      configurable: true,
      value: { documentElement, body },
    });
    Object.defineProperty(globalThis, "window", {
      configurable: true,
      value: {
        matchMedia: vi.fn(() => ({
          matches: false,
          addEventListener: vi.fn(),
          removeEventListener: vi.fn(),
        })),
      },
    });
  });

  afterEach(() => {
    Object.defineProperty(globalThis, "document", {
      configurable: true,
      value: originalDocument,
    });
    Object.defineProperty(globalThis, "window", {
      configurable: true,
      value: originalWindow,
    });
    vi.restoreAllMocks();
  });

  it("maps the default 13px font setting to a visible startup scale", () => {
    applySettingsSideEffects(DEFAULTS);

    expect(document.documentElement.style.getPropertyValue("--ui-scale")).toBe(
      "1.0833333333333333",
    );
    expect(document.body.style.width).toBe("92.30769230769232vw");
    expect(document.body.style.height).toBe("92.30769230769232vh");
    expect((document.body.style as CSSStyleDeclaration & { zoom?: string }).zoom).toBe(
      "1.0833333333333333",
    );
  });
});

function fakeElement() {
  const attributes = new Map<string, string>();
  return {
    style: fakeStyle(),
    setAttribute: (name: string, value: string) => attributes.set(name, value),
  };
}

function fakeStyle() {
  const properties = new Map<string, string>();
  return {
    width: "",
    height: "",
    zoom: "",
    setProperty: (name: string, value: string) => properties.set(name, value),
    getPropertyValue: (name: string) => properties.get(name) ?? "",
  };
}
