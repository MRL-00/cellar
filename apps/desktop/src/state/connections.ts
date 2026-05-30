import { commands, unwrap } from "@cellar/ipc";
import type {
  ConnectionConfig,
  Database,
  DriverInfo,
} from "@cellar/ipc";
import { create } from "zustand";

export type ConnStatus = "connected" | "connecting" | "disconnected" | "error";

interface ConnectionState {
  id: string;
  status: ConnStatus;
  driverInfo: DriverInfo | null;
  error: string | null;
  expanded: boolean;
  databases: Database[];
  loadingSchema: boolean;
}

interface ConnectionsStore {
  connections: ConnectionConfig[];
  byId: Record<string, ConnectionState>;
  loaded: boolean;

  load: () => Promise<void>;
  saveConnection: (
    config: ConnectionConfig,
    password: string | null,
  ) => Promise<ConnectionConfig>;
  deleteConnection: (id: string) => Promise<void>;
  connect: (id: string) => Promise<void>;
  disconnect: (id: string) => Promise<void>;
  toggleExpand: (id: string) => void;
  refreshSchema: (id: string) => Promise<void>;
}

const connectInflight = new Map<string, Promise<void>>();

function emptyState(id: string): ConnectionState {
  return {
    id,
    status: "disconnected",
    driverInfo: null,
    error: null,
    expanded: false,
    databases: [],
    loadingSchema: false,
  };
}

export const useConnections = create<ConnectionsStore>((set, get) => ({
  connections: [],
  byId: {},
  loaded: false,

  async load() {
    const list = await unwrap(commands.listConnections());
    const byId: Record<string, ConnectionState> = {};
    for (const c of list) {
      byId[c.id] = emptyState(c.id);
    }
    set({ connections: list, byId, loaded: true });
  },

  async saveConnection(config, password) {
    const saved = await unwrap(commands.saveConnection(config, password));
    set((s) => {
      const existing = s.connections.find((c) => c.id === saved.id);
      const next = existing
        ? s.connections.map((c) => (c.id === saved.id ? saved : c))
        : [...s.connections, saved];
      return {
        connections: next,
        byId: {
          ...s.byId,
          [saved.id]: s.byId[saved.id] ?? emptyState(saved.id),
        },
      };
    });
    return saved;
  },

  async deleteConnection(id) {
    await unwrap(commands.deleteConnection(id));
    set((s) => {
      const { [id]: _gone, ...rest } = s.byId;
      return {
        connections: s.connections.filter((c) => c.id !== id),
        byId: rest,
      };
    });
  },

  async connect(id) {
    const existing = connectInflight.get(id);
    if (existing) return existing;
    if (get().byId[id]?.status === "connected") return;

    const task = (async () => {
      setStatus(set, id, "connecting", null);
      try {
        const info = await unwrap(commands.connect(id));
        set((s) => ({
          byId: {
            ...s.byId,
            [id]: {
              ...(s.byId[id] ?? emptyState(id)),
              status: "connected",
              driverInfo: info,
              error: null,
            },
          },
        }));
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setStatus(set, id, "error", message);
        throw err;
      } finally {
        connectInflight.delete(id);
      }
    })();
    connectInflight.set(id, task);
    return task;
  },

  async disconnect(id) {
    await unwrap(commands.disconnect(id));
    set((s) => ({
      byId: {
        ...s.byId,
        [id]: {
          ...(s.byId[id] ?? emptyState(id)),
          status: "disconnected",
          driverInfo: null,
          databases: [],
          expanded: false,
        },
      },
    }));
  },

  toggleExpand(id) {
    const state = get().byId[id];
    if (!state) return;
    const willExpand = !state.expanded;
    set((s) => ({
      byId: {
        ...s.byId,
        [id]: { ...(s.byId[id] ?? emptyState(id)), expanded: willExpand },
      },
    }));
    if (willExpand) {
      void onExpand(id);
    }
  },

  async refreshSchema(id) {
    set((s) => ({
      byId: {
        ...s.byId,
        [id]: { ...(s.byId[id] ?? emptyState(id)), loadingSchema: true },
      },
    }));
    try {
      const dbs = await unwrap(commands.introspect(id, true));
      set((s) => ({
        byId: {
          ...s.byId,
          [id]: {
            ...(s.byId[id] ?? emptyState(id)),
            databases: dbs,
            loadingSchema: false,
            error: null,
          },
        },
      }));
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      set((s) => ({
        byId: {
          ...s.byId,
          [id]: {
            ...(s.byId[id] ?? emptyState(id)),
            loadingSchema: false,
            error: message,
          },
        },
      }));
    }
  },
}));

async function onExpand(id: string) {
  const store = useConnections.getState();
  const state = store.byId[id];
  if (!state) return;
  if (state.status !== "connected") {
    try {
      await store.connect(id);
    } catch {
      return;
    }
  }
  const after = useConnections.getState().byId[id];
  if (after && after.databases.length === 0 && !after.loadingSchema) {
    await store.refreshSchema(id);
  }
}

type Setter = (
  partial:
    | ConnectionsStore
    | Partial<ConnectionsStore>
    | ((s: ConnectionsStore) => Partial<ConnectionsStore>),
) => void;

function setStatus(
  set: Setter,
  id: string,
  status: ConnStatus,
  error: string | null,
) {
  set((s) => ({
    byId: {
      ...s.byId,
      [id]: {
        ...(s.byId[id] ?? emptyState(id)),
        status,
        error,
      },
    },
  }));
}
