import { create } from "zustand";
import type { DetectedParameter } from "@cellar/ipc";

import type { ParamValue } from "../lib/queryParamValues";

/** The parameter-collection panel shown below a query editor before a run. */
export interface ParamPanel {
  /** SQL the parameters belong to — the exact statement that will run. */
  sql: string;
  /** Run label surfaced in the Messages panel. */
  label: string;
  errorLine: number | null;
  params: DetectedParameter[];
  /** Current input value per parameter name. */
  values: Record<string, ParamValue>;
  /** Bumped to ask the panel to focus its first empty/invalid input. */
  focusRequest: number;
}

interface QueryParamsStore {
  /** Open panels keyed by tab id. */
  panels: Record<string, ParamPanel>;
  /**
   * Last value used for each parameter name, remembered for the session so
   * re-running (or a different tab using the same name) starts pre-filled.
   */
  remembered: Record<string, ParamValue>;
  openPanel: (tabId: string, panel: Omit<ParamPanel, "focusRequest">) => void;
  setValue: (tabId: string, name: string, patch: Partial<ParamValue>) => void;
  /** Ask the tab's panel to focus its first empty input. */
  requestFocus: (tabId: string) => void;
  remember: (values: Record<string, ParamValue>) => void;
  closePanel: (tabId: string) => void;
}

export const useQueryParams = create<QueryParamsStore>((set) => ({
  panels: {},
  remembered: {},

  openPanel(tabId, panel) {
    set((s) => ({
      panels: { ...s.panels, [tabId]: { ...panel, focusRequest: 0 } },
    }));
  },

  setValue(tabId, name, patch) {
    set((s) => {
      const panel = s.panels[tabId];
      if (!panel) return {};
      const current = panel.values[name] ?? { type: "text", value: "" };
      return {
        panels: {
          ...s.panels,
          [tabId]: {
            ...panel,
            values: { ...panel.values, [name]: { ...current, ...patch } },
          },
        },
      };
    });
  },

  requestFocus(tabId) {
    set((s) => {
      const panel = s.panels[tabId];
      if (!panel) return {};
      return {
        panels: {
          ...s.panels,
          [tabId]: { ...panel, focusRequest: panel.focusRequest + 1 },
        },
      };
    });
  },

  remember(values) {
    set((s) => ({ remembered: { ...s.remembered, ...values } }));
  },

  closePanel(tabId) {
    set((s) => {
      if (!s.panels[tabId]) return {};
      const { [tabId]: _removed, ...panels } = s.panels;
      return { panels };
    });
  },
}));
