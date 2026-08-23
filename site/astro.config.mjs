// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import sitemap from "@astrojs/sitemap";

// GitHub Pages のサブパス配信。CI では BASE_PATH='/WSA_RPC_Bridge/docs/' が渡る。
// 未指定 (ローカル dev 等) ではルート配下で動く。
const base = process.env.BASE_PATH ?? "/";

export default defineConfig({
  base,
  site: "https://hu-ja-ja.github.io",
  trailingSlash: "ignore",
  // 日本語をルートロケールにすることで既存 URL (/docs/guide/...) を維持し、
  // 英語は /en/ 配下に出力する。
  i18n: {
    defaultLocale: "ja",
    locales: ["ja", "en"],
    routing: { prefixDefaultLocale: false },
  },
  integrations: [
    sitemap(),
    starlight({
      // Astro i18n 利用時は言語別キーが必要 (既定ロケールの解決が 'en' のため)
      title: { en: "WSA RPC Bridge", ja: "WSA RPC Bridge" },
      description:
        "WSA / Android のメディア再生情報を Discord Rich Presence に表示するアプリの公式ドキュメント",
      logo: { src: "./src/assets/favicon.svg" },
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/hu-ja-ja/WSA_RPC_Bridge",
        },
      ],
      lastUpdated: true,
      editLink: {
        baseUrl:
          "https://github.com/hu-ja-ja/WSA_RPC_Bridge/edit/main/site/src/content/docs/",
      },
      // slug 指定の項目は各言語のページタイトルを自動で使うため label を付けない
      sidebar: [
        {
          label: "ガイド",
          translations: { en: "Guide" },
          items: [
            { label: "はじめに", link: "/", translations: { en: "Introduction" } },
            { slug: "guide/settings" },
            { slug: "guide/how-it-works" },
            { slug: "guide/troubleshooting" },
            { slug: "guide/android-versions" },
          ],
        },
        {
          label: "開発",
          translations: { en: "Development" },
          items: [
            { slug: "dev/setup" },
            { slug: "dev/commands" },
            { slug: "dev/architecture" },
            { slug: "dev/release" },
          ],
        },
        {
          label: "その他",
          translations: { en: "Other" },
          items: [
            { slug: "legal/privacy" },
            { slug: "legal/terms" },
            { slug: "changelog" },
          ],
        },
      ],
    }),
  ],
});
