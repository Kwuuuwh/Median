import { cpSync, mkdirSync, readFileSync } from "node:fs";
import { build } from "esbuild";

const read = (file) => JSON.parse(readFileSync(file, "utf8"));

// One version, three readers: package.json is the source, the manifest is what Overwolf
// installs by, and the bundle announces itself in `hello`. Drift between them is a build error.
const { version } = read("package.json");
const declared = read("manifest.json").meta.version;
if (declared !== version) {
  throw new Error(`manifest.json says ${declared}, package.json says ${version}`);
}

mkdirSync("dist", { recursive: true });

await build({
  entryPoints: ["src/background.ts"],
  bundle: true,
  outfile: "dist/background.js",
  platform: "browser",
  target: "chrome105",
  format: "iife",
  define: { __COLLECTOR_VERSION__: JSON.stringify(version) },
});

for (const file of [
  "manifest.json",
  "background.html",
  "icon.png",
  "icon_gray.png",
]) {
  cpSync(file, `dist/${file}`);
}

console.log("collector built -> apps/collector/dist/");
