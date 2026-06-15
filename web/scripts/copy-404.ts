import { copyFile } from "node:fs/promises";

const indexPath = new URL("../dist/index.html", import.meta.url);
const notFoundPath = new URL("../dist/404.html", import.meta.url);

try {
  await copyFile(indexPath, notFoundPath);
} catch (error) {
  if (error instanceof Error && "code" in error && error.code === "ENOENT") {
    process.stderr.write(
      "Missing dist/index.html. Run `bun run build` first.\n",
    );
    process.exit(1);
  }
  throw error;
}
