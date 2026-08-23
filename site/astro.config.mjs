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
  integrations: [
    sitemap(),
    starlight({
      title: "WSA RPC Bridge",
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
      sidebar: [
        {
          label: "ガイド",
          items: [
            { label: "はじめに", link: "/" },
            { label: "設定", slug: "guide/settings" },
            { label: "しくみ", slug: "guide/how-it-works" },
            { label: "トラブルシューティング", slug: "guide/troubleshooting" },
            { label: "Android バージョン対応", slug: "guide/android-versions" },
          ],
        },
        {
          label: "開発",
          items: [
            { label: "環境構築", slug: "dev/setup" },
            { label: "コマンドリファレンス", slug: "dev/commands" },
            { label: "アーキテクチャ概要", slug: "dev/architecture" },
            { label: "リリース方針", slug: "dev/release" },
          ],
        },
        {
          label: "その他",
          items: [
            { label: "プライバシーポリシー", slug: "legal/privacy" },
            { label: "利用規約", slug: "legal/terms" },
            { label: "変更履歴", slug: "changelog" },
          ],
        },
      ],
    }),
  ],
});
