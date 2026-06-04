import { commands, IpcError, unwrap } from "@cellar/ipc";
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
  reconnect: (id: string) => Promise<void>;
  disconnect: (id: string) => Promise<void>;
  toggleExpand: (id: string) => void;
  refreshSchema: (id: string) => Promise<void>;
}

const connectInflight = new Map<string, Promise<void>>();
const connectionGenerations = new Map<string, number>();

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
    bumpConnectionGeneration(id);
    connectInflight.delete(id);
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
    return connectWith(set, get, id, false);
  },

  async reconnect(id) {
    return connectWith(set, get, id, true);
  },

  async disconnect(id) {
    bumpConnectionGeneration(id);
    connectInflight.delete(id);
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
      if (noteConnectionIssue(id, err)) return;
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

export function noteConnectionIssue(id: string, err: unknown): boolean {
  if (!isConnectionIssue(err)) return false;
  const message = err instanceof Error ? err.message : String(err);
  useConnections.setState((s) => ({
    byId: {
      ...s.byId,
      ...(s.byId[id]
        ? {
            [id]: {
              ...s.byId[id],
              status: "error",
              driverInfo: null,
              databases: [],
              loadingSchema: false,
              error: message,
            },
          }
        : {}),
    },
  }));
  return true;
}

export function isConnectionIssue(err: unknown): boolean {
  return (
    err instanceof IpcError &&
    (err.kind === "NotConnected" ||
      err.kind === "Connection" ||
      err.kind === "Tls")
  );
}

async function connectWith(
  set: Setter,
  get: () => ConnectionsStore,
  id: string,
  force: boolean,
): Promise<void> {
  const existing = connectInflight.get(id);
  if (existing) return existing;
  if (!force && get().byId[id]?.status === "connected") return;

  const generation = bumpConnectionGeneration(id);
  let task: Promise<void> | null = null;
  task = (async () => {
    const wasExpanded = get().byId[id]?.expanded ?? false;
    setStatus(set, id, "connecting", null);
    try {
      const info = await unwrap(force ? commands.reconnect(id) : commands.connect(id));
      if (!isCurrentGeneration(id, generation) || !get().byId[id]) return;
      set((s) => ({
        byId: {
          ...s.byId,
          [id]: {
            ...(s.byId[id] ?? emptyState(id)),
            status: "connected",
            driverInfo: info,
            error: null,
            databases: force ? [] : (s.byId[id]?.databases ?? []),
          },
        },
      }));
      if (force && wasExpanded) {
        await get().refreshSchema(id);
      }
    } catch (err) {
      if (!isCurrentGeneration(id, generation) || !get().byId[id]) return;
      const message = err instanceof Error ? err.message : String(err);
      setStatus(set, id, "error", message);
      throw err;
    } finally {
      if (task && connectInflight.get(id) === task) {
        connectInflight.delete(id);
      }
    }
  })();
  connectInflight.set(id, task);
  return task;
}

function bumpConnectionGeneration(id: string): number {
  const next = (connectionGenerations.get(id) ?? 0) + 1;
  connectionGenerations.set(id, next);
  return next;
}

function isCurrentGeneration(id: string, generation: number): boolean {
  return connectionGenerations.get(id) === generation;
}

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
      ...(s.byId[id]
        ? {
            [id]: {
              ...s.byId[id],
              status,
              error,
            },
          }
        : {}),
    },
  }));
}
