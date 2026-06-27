import type { DatabaseNotice, NoticeCapture, QueryResult } from "@cellar/ipc";
import { create } from "zustand";

export interface NoticeScope {
  tabId: string | null;
  connectionId: string | null;
  database: string | null;
}

export interface NoticeLogEntry {
  notices: DatabaseNotice[];
  capture: NoticeCapture | null;
  lastQueryAt: string | null;
  retain: boolean;
}

interface NoticeStore {
  byScope: Record<string, NoticeLogEntry>;
  recordQueryResult: (scope: NoticeScope, result: QueryResult) => void;
  appendNotice: (scope: NoticeScope, notice: DatabaseNotice) => void;
  clear: (scope: NoticeScope) => void;
  setRetain: (scope: NoticeScope, retain: boolean) => void;
  /** Remove every tab-scoped entry for the given tabs (used on tab close). */
  dropTabs: (tabIds: string[]) => void;
}

export const EMPTY_ENTRY: NoticeLogEntry = {
  notices: [],
  capture: null,
  lastQueryAt: null,
  retain: true,
};

// Cap retained notices so a long-lived tab that keeps querying (e.g. typing in
// the quick filter, one query per keystroke) can't grow this array without
// bound. Mirrors trimMessages() in queryMessages.ts.
const MAX_NOTICES = 200;

function trimNotices(notices: DatabaseNotice[]): DatabaseNotice[] {
  if (notices.length <= MAX_NOTICES) return notices;
  return notices.slice(notices.length - MAX_NOTICES);
}

export function noticeScopeKey(scope: NoticeScope): string {
  if (scope.tabId) return `tab:${scope.tabId}`;
  if (scope.connectionId) {
    return `connection:${scope.connectionId}:${scope.database ?? ""}`;
  }
  return "global";
}

// ponytail: emptyNoticeEntry() factory dropped; use structuredClone(EMPTY_ENTRY) at call sites
export const useNotices = create<NoticeStore>((set) => ({
  byScope: {},

  recordQueryResult(scope, result) {
    const key = noticeScopeKey(scope);
    set((s) => {
      const previous = s.byScope[key] ?? structuredClone(EMPTY_ENTRY);
      const notices = previous.retain
        ? trimNotices([...previous.notices, ...result.notices])
        : result.notices;
      return {
        byScope: {
          ...s.byScope,
          [key]: {
            notices,
            capture: result.notice_capture,
            lastQueryAt: new Date().toISOString(),
            retain: previous.retain,
          },
        },
      };
    });
  },

  appendNotice(scope, notice) {
    const key = noticeScopeKey(scope);
    set((s) => {
      const previous = s.byScope[key] ?? structuredClone(EMPTY_ENTRY);
      return {
        byScope: {
          ...s.byScope,
          [key]: {
            ...previous,
            notices: trimNotices([...previous.notices, notice]),
          },
        },
      };
    });
  },

  clear(scope) {
    const key = noticeScopeKey(scope);
    set((s) => {
      const previous = s.byScope[key] ?? structuredClone(EMPTY_ENTRY);
      return {
        byScope: {
          ...s.byScope,
          [key]: {
            ...previous,
            notices: [],
          },
        },
      };
    });
  },

  setRetain(scope, retain) {
    const key = noticeScopeKey(scope);
    set((s) => {
      const previous = s.byScope[key] ?? structuredClone(EMPTY_ENTRY);
      return {
        byScope: {
          ...s.byScope,
          [key]: {
            ...previous,
            retain,
          },
        },
      };
    });
  },

  dropTabs(tabIds) {
    if (tabIds.length === 0) return;
    const dropped = new Set(
      tabIds.map((tabId) =>
        noticeScopeKey({ tabId, connectionId: null, database: null }),
      ),
    );
    set((s) => ({
      byScope: Object.fromEntries(
        Object.entries(s.byScope).filter(([key]) => !dropped.has(key)),
      ),
    }));
  },
}));
