import { readFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { join } from "node:path";

// astro build 後に dist/ 内の全 HTML について、内部リンク (<a href> / <img src>)
// をページ URL 基準で実際に解決し、対応ファイルが存在するかを検証する。
// アンカー (#) は解決先 HTML 内の id 属性と突き合わせる。
const distDir = new URL("../dist", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
// astro.config.mjs と同じ環境変数で base を決める
const BASE = process.env.BASE_PATH ?? "/";

if (!existsSync(distDir)) {
  console.error("dist/ not found. Run `astro build` first.");
  process.exit(1);
}

function walk(dir) {
  const out = [];
  for (const name of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, name.name);
    if (name.isDirectory()) out.push(...walk(p));
    else out.push(p);
  }
  return out;
}

/** URL パスから dist 内の実ファイルを探す */
function resolveFile(urlPath) {
  let p = urlPath.split("?")[0].split("#")[0];
  if (!p.endsWith("/")) {
    // ディレクトリは index.html 解決が必要なためファイルのみ早期リターン
    const direct = join(distDir, p);
    if (existsSync(direct) && statSync(direct).isFile()) return p;
    p += "/";
  }
  const idx = join(distDir, p, "index.html").replace(/\\/g, "/");
  return existsSync(idx) ? `${p}/index.html`.replace(/^\//, "") : null;
}

function normalize(pathname) {
  const parts = [];
  for (const seg of pathname.split("/")) {
    if (seg === "" || seg === ".") continue;
    if (seg === "..") parts.pop();
    else parts.push(seg);
  }
  return `/${parts.join("/")}`;
}

/** href 中のフラグメントは % エンコードされるため id と比較前にデコードする */
function decodeHash(hash) {
  try {
    return decodeURIComponent(hash);
  } catch {
    return hash;
  }
}

const files = walk(distDir);
const htmlFiles = files.filter((f) => f.endsWith(".html"));
const errors = [];

for (const file of htmlFiles) {
  const rel = file.slice(distDir.length + 1).replace(/\\/g, "/");
  // ページの基準ディレクトリ (相対リンクの解決基準)。directory 形式の URL を想定
  const pageDir =
    rel === "index.html"
      ? BASE
      : /\/index\.html$/.test(rel)
        ? BASE + rel.slice(0, -"index.html".length)
        : BASE + rel.slice(0, rel.lastIndexOf("/") + 1);
  const html = readFileSync(file, "utf8");

  for (const [, attr, raw] of html.matchAll(/\b(href|src)="([^"]+)"/g)) {
    if (/^(https?:|mailto:)/.test(raw) || raw.startsWith("//")) continue;
    // 絶対パス・相対パスともに base を剥がして dist 相対にする
    const joined = raw.startsWith("/") ? raw : pageDir + raw;
    const [pathname, hash] = joined.split("#", 2);
    const target = normalize(
      pathname.startsWith(BASE) ? pathname.slice(BASE.length - 1) : pathname
    );

    // 同一ページ内アンカー
    if (raw.startsWith("#")) {
      const id = decodeHash(raw.slice(1));
      if (!html.includes(`id="${id}"`)) {
        errors.push(`${rel}: broken anchor ${raw}`);
      }
      continue;
    }

    const resolved = resolveFile(target);
    if (!resolved) {
      errors.push(`${rel}: ${attr}="${raw}" -> ${target} (not found)`);
      continue;
    }

    // アンカー付きリンクは解決先の id を確認
    if (hash && resolved.endsWith(".html")) {
      const dest = readFileSync(join(distDir, resolved), "utf8");
      if (!dest.includes(`id="${decodeHash(hash)}"`)) {
        errors.push(`${rel}: anchor #${hash} not found in ${resolved}`);
      }
    }
  }
}

if (errors.length > 0) {
  console.error(`link-check failed (${errors.length}):\n` + errors.map((e) => `  - ${e}`).join("\n"));
  process.exit(1);
}
console.log(`link-check OK: ${htmlFiles.length} pages`);
