import { FluentBundle, FluentResource, type FluentVariable } from "@fluent/bundle";
import type { MessageId } from "./ids";
import { DEFAULT_LOCALE, type Locale } from "./locale";
import en from "./locales/en.ftl?raw";
import ru from "./locales/ru.ftl?raw";

/// Arguments of a message.
export type Args = Record<string, FluentVariable>;

const bundles = new Map<Locale, FluentBundle>();

function create(locale: Locale): FluentBundle | null {
  try {
    // Rejects a tag Intl cannot parse before it reaches plural rules or number formats.
    Intl.getCanonicalLocales(locale);
  } catch {
    console.error(`${locale} is not a language tag Intl understands`);
    return null;
  }
  // Isolation marks guard mixed-direction text; here they would only be stray characters.
  return new FluentBundle(locale, { useIsolating: false });
}

function load(locale: Locale, source: string): void {
  let bundle = bundles.get(locale);
  if (!bundle) {
    const created = create(locale);
    if (!created) return;
    bundle = created;
    bundles.set(locale, bundle);
  }

  const failed = bundle.addResource(new FluentResource(source));
  for (const error of failed) console.error(`${locale}: ${error.message}`);
}

/// Message source of each interface language.
const SOURCES: Readonly<Record<Locale, string>> = { ru, en };

/// Interface languages, in the order the settings offer them.
export const LOCALES: readonly Locale[] = Object.keys(SOURCES);

for (const [locale, source] of Object.entries(SOURCES)) load(locale, source);

function format(locale: Locale, id: MessageId, args?: Args): string | null {
  const bundle = bundles.get(locale);
  const message = bundle?.getMessage(id);
  if (!bundle || !message?.value) return null;

  const errors: Error[] = [];
  const text = bundle.formatPattern(message.value, args, errors);
  for (const error of errors) console.error(`${id}: ${error.message}`);
  return text;
}

/// The message in the given locale, falling back to the default locale, then to the message id.
export function translate(locale: Locale, id: MessageId, args?: Args): string {
  return format(locale, id, args) ?? format(DEFAULT_LOCALE, id, args) ?? id;
}
