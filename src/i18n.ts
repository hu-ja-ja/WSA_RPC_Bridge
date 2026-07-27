import { flatten, translator, resolveTemplate } from "@solid-primitives/i18n"
import type { Translator } from "@solid-primitives/i18n"
import ja from "./locales/ja.json"
import en from "./locales/en.json"

type Dict = typeof ja
type T = Translator<ReturnType<typeof flatten<Dict>>>

const dicts: Record<string, Dict> = { ja, en }

function detectLocale(): string {
  const lang = navigator.language
  if (lang === "ja" || lang.startsWith("ja-")) return "ja"
  return "en"
}

export const locale = detectLocale()

const dict = flatten(dicts[locale])
export const t: T = translator(() => dict, resolveTemplate)
