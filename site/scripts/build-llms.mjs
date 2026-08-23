import { readFileSync, readdirSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";

// astro build 後に dist/ へ llms.txt / llms-full.txt を生成する。
// 法務ページ (legal/) は旧サイトの llms.exclude と同様に対象外。
// 英語版 (en/) も対象外 (日本語のみを出力)。
const docsDir = new URL("../src/content/docs", import.meta.url).pathname
  .replace(/^\/([A-Za-z]:)/, "$1");
const distDir = new URL("../dist", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");

const SITE = "https://hu-ja-ja.github.io/WSA_RPC_Bridge/docs/";

function walk(dir) {
  const out = [];
  for (const name of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, name.name);
    if (name.isDirectory()) out.push(...walk(p));
    else if (/\.mdx?$/.test(name.name)) out.push(p);
  }
  return out;
}

function slugOf(file) {
  return file
    .slice(docsDir.length + 1)
    .replace(/\\/g, "/")
    .replace(/\.mdx?$/, "")
    .replace(/(^|\/)index$/, "");
}

function stripFrontmatter(src) {
  const m = src.match(/^---\r?\n[\s\S]*?\r?\n---\r?\n/);
  return m ? src.slice(m[0].length) : src;
}

function meta(src) {
  const title = src.match(/^title:\s*(.+)$/m)?.[1]?.trim() ?? "";
  const desc = src.match(/^description:\s*(.+)$/m)?.[1]?.trim() ?? "";
  return { title, desc };
}

if (!existsSync(distDir)) {
  console.error("dist/ not found. Run `astro build` first.");
  process.exit(1);
}

const files = walk(docsDir)
  .filter((f) => {
    const p = f.replace(/\\/g, "/");
    return !p.includes("/legal/") && !p.includes("/en/");
  })
  .sort((a, b) => (slugOf(a) === "index" ? -1 : a.localeCompare(b)));

const full = [];
const index = ["# WSA RPC Bridge", "", "> WSA / Android のメディア再生情報を Discord Rich Presence に表示するアプリの公式ドキュメント", "", "## Docs", ""];

for (const file of files) {
  const src = readFileSync(file, "utf8");
  const { title, desc } = meta(src);
  const slug = slugOf(file);
  const url = `${SITE}${slug ? `${slug}/` : ""}`;
  index.push(`- [${title}](${url})${desc ? `: ${desc}` : ""}`);
  full.push(`# ${title}\n\n${stripFrontmatter(src).replace(/\r\n/g, "\n").trim()}\n`);
}

writeFileSync(join(distDir, "llms.txt"), index.join("\n") + "\n");
writeFileSync(join(distDir, "llms-full.txt"), full.join("\n---\n\n"));
console.log(`llms.txt generated (${files.length} pages)`);
