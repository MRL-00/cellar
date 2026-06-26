import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { deferredStorage } from "./deferredStorage";

export type Panels = { left: boolean; right: boolean; bottom: boolean };
export type PanelId = keyof Panels;

type LayoutStore = {
  panels: Panels;
  leftWidth: number;
  rightWidth: number;
  bottomHeight: number;
  togglePanel: (id: PanelId) => void;
  setPanel: (id: PanelId, open: boolean) => void;
  setLeftWidth: (width: number) => void;
  setRightWidth: (width: number) => void;
  setBottomHeight: (height: number) => void;
};

const DEFAULT_PANELS: Panels = {
  left: true,
  right: false,
  bottom: false,
};

export const useLayout = create<LayoutStore>()(
  persist(
    (set) => ({
      panels: DEFAULT_PANELS,
      leftWidth: 256,
      rightWidth: 380,
      bottomHeight: 280,
      togglePanel: (id) =>
        set((s) => ({ panels: { ...s.panels, [id]: !s.panels[id] } })),
      setPanel: (id, open) =>
        set((s) => ({ panels: { ...s.panels, [id]: open } })),
      setLeftWidth: (leftWidth) => set({ leftWidth }),
      setRightWidth: (rightWidth) => set({ rightWidth }),
      setBottomHeight: (bottomHeight) => set({ bottomHeight }),
    }),
    {
      name: "cellar.layout.v1",
      storage: createJSONStorage(() => deferredStorage),
      partialize: (s) => ({
        panels: s.panels,
        leftWidth: s.leftWidth,
        rightWidth: s.rightWidth,
        bottomHeight: s.bottomHeight,
      }),
    },
  ),
);
