import { FluentBundle, FluentResource } from "@fluent/bundle";
import { readdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const locales = join(root, "src", "locales");
const target = join(root, "src", "ids.ts");
const reference = "ru";

/// Message ids of an FTL source: an identifier at the start of a line, before `=`.
/// Continuation lines are indented, so anchoring to the line start is enough.
function ids(source) {
  const found = [...source.matchAll(/^([a-zA-Z][a-zA-Z0-9_-]*) *=/gm)];
  return found.map((match) => match[1]).sort();
}

/// Ids the runtime parser refused: it drops what it cannot read without a word.
function unreadable(source, list) {
  const bundle = new FluentBundle("en", { useIsolating: false });
  bundle.addResource(new FluentResource(source));
  return list.filter((id) => !bundle.hasMessage(id));
}

/// Message bodies of an FTL source, keyed by id: the text after `=` and every line
/// indented under it.
function bodies(source) {
  const found = {};
  let id = null;
  for (const line of source.split(/\r?\n/)) {
    const start = line.match(/^([a-zA-Z][a-zA-Z0-9_-]*) *= *(.*)$/);
    if (start) {
      id = start[1];
      found[id] = start[2];
    } else if (id && /^\s+\S/.test(line)) {
      found[id] += "\n" + line;
    } else if (line.trim() === "") {
      id = null;
    }
  }
  return found;
}

/// Branch keys that stand for a plural form. A language has the ones it has, so they
/// are left out of the comparison; every other branch key must match.
const PLURAL = ["zero", "one", "two", "few", "many", "other"];

/// What a message takes and what it branches on.
function shape(body) {
  const args = [...new Set([...body.matchAll(/\$([a-zA-Z][a-zA-Z0-9_]*)/g)].map((m) => m[1]))];
  const branches = [...body.matchAll(/^\s*\*?\[([^\]]+)\]/gm)]
    .map((match) => match[1])
    .filter((key) => !PLURAL.includes(key));
  return `${args.sort().join(",")} | ${[...new Set(branches)].sort().join(",")}`;
}

function render(list) {
  const union = list.map((id) => `  | "${id}"`).join("\n");
  return [
    `// Generated from src/locales/${reference}.ftl by scripts/ids.mjs. Do not edit.`,
    "",
    "/// Every message the interface can ask for.",
    "export type MessageId =",
    union + ";",
    "",
  ].join("\n");
}

const files = (await readdir(locales)).filter((name) => name.endsWith(".ftl"));
const sets = new Map();
const shapes = new Map();
let parity = true;
for (const file of files) {
  const locale = file.replace(".ftl", "");
  const source = await readFile(join(locales, file), "utf8");
  const list = ids(source);

  const junk = unreadable(source, list);
  if (junk.length > 0) {
    console.error(`${file} declares what Fluent cannot read: ${junk.join(", ")}`);
    parity = false;
  }

  sets.set(locale, list);
  shapes.set(locale, bodies(source));
}

const expected = sets.get(reference);
if (!expected) {
  console.error(`no ${reference}.ftl to take the ids from`);
  process.exit(1);
}

for (const [locale, list] of sets) {
  if (locale === reference) continue;
  const missing = expected.filter((id) => !list.includes(id));
  const extra = list.filter((id) => !expected.includes(id));
  if (missing.length > 0) {
    console.error(`${locale}.ftl is missing: ${missing.join(", ")}`);
    parity = false;
  }
  if (extra.length > 0) {
    console.error(`${locale}.ftl has no counterpart in ${reference}.ftl: ${extra.join(", ")}`);
    parity = false;
  }
}

const canon = shapes.get(reference);
for (const [locale, found] of shapes) {
  if (locale === reference) continue;
  for (const [id, body] of Object.entries(found)) {
    if (canon[id] === undefined) continue;
    if (shape(body) !== shape(canon[id])) {
      console.error(`${locale}.ftl: ${id} takes something else than ${reference}.ftl does`);
      parity = false;
    }
  }
}
if (!parity) process.exit(1);

const wanted = render(expected);
if (process.argv.includes("--check")) {
  const current = await readFile(target, "utf8").catch(() => "");
  if (current !== wanted) {
    console.error("src/ids.ts is stale; run `pnpm --filter @median/i18n ids`");
    process.exit(1);
  }
  console.log(`${expected.length} messages, locales in parity`);
} else {
  await writeFile(target, wanted);
  console.log(`wrote src/ids.ts (${expected.length} messages)`);
}
