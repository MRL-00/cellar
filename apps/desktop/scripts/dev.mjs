import net from "node:net";
import { spawn } from "node:child_process";

const DEFAULT_PORT = 1430;
const requestedPort = Number(process.env.CELLAR_DEV_PORT ?? DEFAULT_PORT);
const devPort = await findAvailablePort(requestedPort);

if (devPort !== requestedPort) {
  console.log(
    `Port ${requestedPort} is in use; starting Cellar dev server on ${devPort}.`,
  );
}

const config = JSON.stringify({
  build: {
    devUrl: `http://localhost:${devPort}`,
  },
});

const child = spawn("pnpm", ["exec", "tauri", "dev", "--config", config], {
  cwd: new URL("..", import.meta.url),
  env: {
    ...process.env,
    CELLAR_DEV_PORT: String(devPort),
  },
  stdio: "inherit",
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 0);
});

async function findAvailablePort(start) {
  let port = start;
  while (!(await canUsePort(port))) {
    port += 1;
  }
  return port;
}

async function canUsePort(port) {
  const results = await Promise.all([
    canListen(port, "127.0.0.1"),
    canListen(port, "::1"),
  ]);
  return results.every(Boolean);
}

function canListen(port, host) {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.once("error", (error) => {
      if (error.code === "EADDRINUSE" || error.code === "EACCES") {
        resolve(false);
        return;
      }
      if (host === "::1" && error.code === "EADDRNOTAVAIL") {
        resolve(true);
        return;
      }
      reject(error);
    });
    server.listen({ host, port }, () => {
      server.close(() => resolve(true));
    });
  });
}
