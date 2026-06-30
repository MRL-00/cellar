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

  it("maps the default 13.5px font setting to a comfortable startup zoom", () => {
    applySettingsSideEffects(DEFAULTS);

    expect(document.documentElement.style.getPropertyValue("--ui-scale")).toBe(
      "1.125",
    );
    expect(document.body.style.width).toBe("88.88888888888889vw");
    expect(document.body.style.height).toBe("88.88888888888889vh");
    expect((document.body.style as CSSStyleDeclaration & { zoom?: string }).zoom).toBe(
      "1.125",
    );
  });

  it("picks a legible --accent-fg by contrast, not brightness", () => {
    const fg = (accent: string) => {
      applySettingsSideEffects({ ...DEFAULTS, accent });
      return document.documentElement.style.getPropertyValue("--accent-fg");
    };
    // Mid neutral gray: dark text (~6:1) beats white (~3.5:1).
    expect(fg("#8a8a8a")).toBe("#0a0b0e");
    // Very dark accent needs white text.
    expect(fg("#000000")).toBe("#ffffff");
    // Light accent keeps dark text.
    expect(fg("#fbbf24")).toBe("#0a0b0e");
  });

  it("nudges an invisible accent into a visible range", () => {
    const accentVar = (accent: string) => {
      applySettingsSideEffects({ ...DEFAULTS, accent });
      return document.documentElement.style.getPropertyValue("--accent");
    };
    // Black has no contrast against the dark theme bg → brightened to a gray.
    expect(accentVar("#000000")).not.toBe("#000000");
    // A vivid accent already clears the threshold → left untouched.
    expect(accentVar("#a78bfa")).toBe("#a78bfa");
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
