import { type Args, type MessageId, translate } from "@median/i18n";
import { useLocale } from "@/shared/data/app-state";

/** Translates a message id with arguments into the interface language. */
export type Translate = (id: MessageId, args?: Args) => string;

/** Translator for the interface language set in the app state. */
export function useT(): Translate {
  const locale = useLocale();
  return (id, args) => translate(locale, id, args);
}
