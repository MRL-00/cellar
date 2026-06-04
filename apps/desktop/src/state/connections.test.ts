import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CellarError, DriverInfo, Result } from "@cellar/ipc";

const ipc = vi.hoisted(() => ({
  connect: vi.fn(),
  deleteConnection: vi.fn(),
  disconnect: vi.fn(),
  introspect: vi.fn(),
  listConnections: vi.fn(),
  reconnect: vi.fn(),
  saveConnection: vi.fn(),
}));

vi.mock("@cellar/ipc", () => ({
  commands: ipc,
  IpcError: class IpcError extends Error {
    override readonly name = "IpcError";
    constructor(
      readonly kind: string,
      readonly detail: string,
    ) {
      super(`${kind}: ${detail}`);
    }
  },
  unwrap: async <T,>(promise: Promise<Result<T, CellarError>>): Promise<T> => {
    const result = await promise;
    if (result.status === "ok") return result.data;
    throw new Error(`${result.error.kind}: ${result.error.detail}`);
  },
}));

import { useConnections } from "./connections";

const driverInfo: DriverInfo = {
  engine: "postgres",
  version: "PostgreSQL 16 test",
};

const config = {
  id: "conn-race",
  name: "Race DB",
  engine: "postgres",
  host: "localhost",
  port: 5432,
  database: "app",
  user: "postgres",
  ssl_mode: "prefer",
  env_tag: null,
  application_name: null,
  color: null,
} as const;

describe("connection state", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    ipc.listConnections.mockResolvedValue(ok([config]));
    ipc.disconnect.mockResolvedValue(ok(null));
    ipc.deleteConnection.mockResolvedValue(ok(null));
    ipc.introspect.mockResolvedValue(ok([]));
    useConnections.setState({
      connections: [],
      byId: {},
      loaded: false,
    });
    await useConnections.getState().load();
  });

  it("does not reconnect after disconnect wins a slow connect race", async () => {
    const deferredConnect = deferred<Result<DriverInfo, CellarError>>();
    ipc.connect.mockReturnValueOnce(deferredConnect.promise);

    const connect = useConnections.getState().connect(config.id);
    expect(useConnections.getState().byId[config.id]?.status).toBe("connecting");

    await useConnections.getState().disconnect(config.id);
    deferredConnect.resolve(ok(driverInfo));
    await connect;

    expect(useConnections.getState().byId[config.id]?.status).toBe("disconnected");
  });

  it("does not recreate a deleted connection after a slow connect resolves", async () => {
    const deferredConnect = deferred<Result<DriverInfo, CellarError>>();
    ipc.connect.mockReturnValueOnce(deferredConnect.promise);

    const connect = useConnections.getState().connect(config.id);
    await useConnections.getState().deleteConnection(config.id);
    deferredConnect.resolve(ok(driverInfo));
    await connect;

    expect(useConnections.getState().byId[config.id]).toBeUndefined();
  });
});

function ok<T>(data: T): Result<T, CellarError> {
  return { status: "ok", data };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}
