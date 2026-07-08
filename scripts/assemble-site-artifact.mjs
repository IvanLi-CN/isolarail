import {
  access,
  copyFile,
  cp,
  mkdir,
  rm,
  writeFile,
} from "node:fs/promises";
import { constants } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outputDir = path.resolve(rootDir, process.env.PUBLISH_DIR ?? ".site-publish");
const webDistDir = path.resolve(rootDir, process.env.WEB_DIST_DIR ?? "web/dist");
const docsDistDir = path.resolve(
  rootDir,
  process.env.DOCS_DIST_DIR ?? "docs-site/doc_build",
);
const docsSubdir = (process.env.DOCS_SUBDIR ?? "docs").replace(
  /^\/+|\/+$/g,
  "",
);
const cnameSource = path.resolve(
  rootDir,
  process.env.CNAME_SOURCE ?? "docs-site/docs/public/CNAME",
);

async function exists(targetPath) {
  try {
    await access(targetPath, constants.F_OK);
    return true;
  } catch {
    return false;
  }
}

await rm(outputDir, { force: true, recursive: true });
await mkdir(outputDir, { recursive: true });

await cp(webDistDir, outputDir, { recursive: true });

const docsOutputDir = path.join(outputDir, docsSubdir);
await mkdir(docsOutputDir, { recursive: true });
await cp(docsDistDir, docsOutputDir, { recursive: true });
await rm(path.join(docsOutputDir, "CNAME"), { force: true });

if (await exists(cnameSource)) {
  await copyFile(cnameSource, path.join(outputDir, "CNAME"));
}

await writeFile(path.join(outputDir, ".nojekyll"), "");
await writeFile(
  path.join(outputDir, "package.json"),
  JSON.stringify(
    {
      name: "isolarail-site",
      private: true,
    },
    null,
    2,
  ) + "\n",
);

process.stdout.write(
  `Assembled combined publish artifact at ${path.relative(rootDir, outputDir)}\n`,
);
