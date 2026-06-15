import { createServer } from "node:net";

async function getAvailablePort(): Promise<number> {
  return await new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => {
        if (address && typeof address === "object") {
          resolve(address.port);
        } else {
          reject(new Error("Could not allocate Storybook test port"));
        }
      });
    });
  });
}

async function waitForServer(url: string): Promise<void> {
  const deadline = Date.now() + 30_000;
  let lastError: unknown;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
      lastError = new Error(`HTTP ${response.status}`);
    } catch (error) {
      lastError = error;
    }
    await Bun.sleep(250);
  }
  throw lastError instanceof Error
    ? lastError
    : new Error("Storybook server did not become ready");
}

const port = await getAvailablePort();
const url = `http://127.0.0.1:${port}`;
const server = Bun.spawn(
  [
    "bunx",
    "http-server",
    "storybook-static",
    "--host",
    "127.0.0.1",
    "--port",
    String(port),
    "--silent",
  ],
  {
    stdout: "inherit",
    stderr: "inherit",
  },
);

const stopServer = () => {
  if (!server.killed) {
    server.kill("SIGTERM");
  }
};

process.on("SIGINT", () => {
  stopServer();
  process.exit(130);
});
process.on("SIGTERM", () => {
  stopServer();
  process.exit(143);
});

try {
  await waitForServer(url);
  const tests = Bun.spawn(["bunx", "test-storybook", "--url", url, "--ci"], {
    stdout: "inherit",
    stderr: "inherit",
  });
  const code = await tests.exited;
  stopServer();
  await server.exited.catch(() => undefined);
  process.exit(code);
} catch (error) {
  stopServer();
  await server.exited.catch(() => undefined);
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
}
