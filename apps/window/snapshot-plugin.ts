import { createReadStream, existsSync, readdirSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";
import type { Plugin } from "vite";

const MODULE = "virtual:median-snapshot";
const RESOLVED = `\0${MODULE}`;

/** Serves the snapshot in `MEDIAN_SNAPSHOT`: its recorded calls as a module, its pictures under `/pictures`. */
export function snapshotPlugin(): Plugin {
  const folder = process.env.MEDIAN_SNAPSHOT;
  return {
    name: "median-snapshot",
    resolveId: (id) => (id === MODULE ? RESOLVED : undefined),
    configureServer(server) {
      server.middlewares.use("/pictures", (request, response, next) => {
        const name = basename(new URL(request.url ?? "", "http://stand").pathname);
        const file = folder && join(folder, "pictures", name);
        if (!file || !existsSync(file)) return next();
        response.setHeader("Content-Type", "image/webp");
        createReadStream(file).pipe(response);
      });
    },
    load(id) {
      if (id !== RESOLVED) return undefined;
      const recorded: Record<string, unknown> = {};
      if (folder) {
        for (const file of readdirSync(folder).filter((name) => name.endsWith(".json"))) {
          this.addWatchFile(join(folder, file));
          recorded[basename(file, ".json")] = JSON.parse(readFileSync(join(folder, file), "utf8"));
        }
      }
      return `export default ${JSON.stringify(recorded)};`;
    },
  };
}
