#!/usr/bin/env node

/**
 * Scrapes grammar data from public Japanese grammar resources into SQLite.
 *
 * Usage:
 *   node scripts/scrape-grammar.mjs                                        # scrape all sources
 *   node scripts/scrape-grammar.mjs --source mainichi-nonbiri              # scrape one source
 *   node scripts/scrape-grammar.mjs --source nihongokyoshi-net
 *   node scripts/scrape-grammar.mjs --url https://mainichi-nonbiri.com/grammar/n0-aguneru/
 *   node scripts/scrape-grammar.mjs --url https://mainichi-nonbiri.com/grammar/n1-atteno/ --dry-run
 */

import { existsSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import Database from "better-sqlite3";
import { parse as parseHTML } from "node-html-parser";

const __dirname = dirname(fileURLToPath(import.meta.url));
const DB_PATH = resolve(__dirname, "..", "sources", "grammars.sqlite");

// ── CLI args ──────────────────────────────────────────────────────────────

const args = process.argv.slice(2);
let sourceFilter = null;
let singleUrl = null;
let dryRun = false;
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--source" && args[i + 1]) {
    sourceFilter = args[i + 1];
    i++;
  } else if (args[i] === "--url" && args[i + 1]) {
    singleUrl = args[i + 1];
    i++;
  } else if (args[i] === "--dry-run") {
    dryRun = true;
  }
}

// ── Database setup ────────────────────────────────────────────────────────

function initDB() {
  const dir = dirname(DB_PATH);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });

  const db = new Database(DB_PATH);
  db.pragma("journal_mode = WAL");

  db.exec(`
    CREATE TABLE IF NOT EXISTS grammars (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      rule_name TEXT UNIQUE NOT NULL,
      levels TEXT NOT NULL,
      name TEXT NOT NULL,
      description TEXT,
      connection TEXT,
      pattern TEXT,
      source TEXT,
      source_url TEXT,
      created_at TEXT DEFAULT (datetime('now')),
      updated_at TEXT DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS examples (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      grammar_id INTEGER NOT NULL REFERENCES grammars(id),
      sentence TEXT NOT NULL,
      lang TEXT NOT NULL DEFAULT 'ja',
      sort_order INTEGER DEFAULT 0
    );
  `);

  // Migrate: add updated_at if missing
  const cols = db.prepare("PRAGMA table_info(grammars)").all();
  if (!cols.some((c) => c.name === "updated_at")) {
    db.exec(
      "ALTER TABLE grammars ADD COLUMN updated_at TEXT DEFAULT (datetime('now'))",
    );
  }

  return db;
}

// ── Helpers ───────────────────────────────────────────────────────────────

/** Rate-limit HTTP requests */
function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

/** Fetch a URL with retry */
async function fetchPage(url, retries = 5) {
  for (let i = 0; i < retries; i++) {
    try {
      const res = await fetch(url, {
        headers: {
          "User-Agent":
            "Mozilla/5.0 (compatible; GrammarScraper/1.0; +educational-use)",
          Accept: "text/html",
        },
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return await res.text();
    } catch (e) {
      const backoff = Math.min(5000 * 2 ** i, 120000); // 5s, 10s, 20s, 40s, 80s (cap 120s)
      console.error(`  Fetch error (attempt ${i + 1}/${retries}): ${e.message} — retry in ${backoff / 1000}s`);
      if (i < retries - 1) await sleep(backoff);
    }
  }
  return null;
}

/** Extract rule_name from URL slug */
function toRuleNameFromUrl(url) {
  const match = url.match(/\/([^/]+)\/?$/);
  if (match) {
    return match[1].replace(/-/g, "_");
  }
  return `grammar_${Buffer.from(url).toString("hex").slice(0, 12)}`;
}

/**
 * Upsert a grammar record and replace its examples.
 * Returns the grammar id.
 */
function upsertGrammar(db, { ruleName, levels, name, description, connection, pattern, source, sourceUrl, examples }) {
  const existing = db
    .prepare("SELECT id FROM grammars WHERE rule_name = ?")
    .get(ruleName);

  let grammarId;
  if (existing) {
    db.prepare(
      `UPDATE grammars SET levels = ?, name = ?, description = ?, connection = ?, pattern = ?, source = ?, source_url = ?, updated_at = datetime('now')
       WHERE rule_name = ?`,
    ).run(levels, name, description, connection, pattern, source, sourceUrl, ruleName);
    grammarId = existing.id;
  } else {
    const result = db
      .prepare(
        `INSERT INTO grammars (rule_name, levels, name, description, connection, pattern, source, source_url)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
      )
      .run(ruleName, levels, name, description, connection, pattern, source, sourceUrl);
    grammarId = result.lastInsertRowid;
  }

  // Replace examples
  if (examples && examples.length > 0) {
    db.prepare("DELETE FROM examples WHERE grammar_id = ?").run(grammarId);
    const insertExample = db.prepare(
      "INSERT INTO examples (grammar_id, sentence, lang, sort_order) VALUES (?, ?, ?, ?)",
    );
    for (let i = 0; i < examples.length; i++) {
      const ex = examples[i];
      if (typeof ex === "string") {
        // Simple string example (nihongokyoshi-net style)
        insertExample.run(grammarId, ex, "ja", i);
      } else {
        // Structured example with translations (mainichi style)
        insertExample.run(grammarId, ex.sentence, "ja", i);
        for (const t of ex.translations) {
          insertExample.run(grammarId, t.text, t.lang, i);
        }
      }
    }
  }

  return grammarId;
}

/** Detect source type from URL */
function detectSource(url) {
  if (url.includes("mainichi-nonbiri")) return "mainichi_nonbiri";
  if (url.includes("nihongokyoshi-net")) return "nihongokyoshi_net";
  return "unknown";
}

/** Full-width digit to half-width */
const FW_MAP = { "０": "0", "１": "1", "２": "2", "３": "3", "４": "4", "５": "5", "６": "6", "７": "7", "８": "8", "９": "9" };
function fw2hw(s) {
  return s.replace(/[０-９]/g, (c) => FW_MAP[c] || c);
}

// ── Page parser: mainichi-nonbiri ─────────────────────────────────────────

/**
 * Parse a single mainichi-nonbiri grammar detail page.
 * Returns { ruleName, levels, name, description, connection, pattern, examples }
 */
function parseMainichiPage(html, url) {
  const page = parseHTML(html);

  // Extract grammar name from h1
  const title =
    page.querySelector("h1")?.text?.trim() ||
    page.querySelector("title")?.text?.trim() ||
    "";

  // Extract JLPT level and clean name from title like "【Ｎ３文法】～あまり"
  let grammarName = title
    .replace(/\s*[|｜].*/g, "")
    .replace(/の意味.*/, "")
    .trim();
  let levels = "";
  const levelInName = grammarName.match(/【(Ｎ[０-９])文法】/);
  if (levelInName) {
    levels = fw2hw(levelInName[1]).replace("Ｎ", "N");
    grammarName = grammarName.replace(/【Ｎ[０-９]文法】/, "").trim();
  } else {
    const levelMatch =
      html.match(/JLPT\s*(N[1-5])/i) || html.match(/(N[1-5])/);
    levels = levelMatch ? levelMatch[1] : "";
  }

  // Extract description (解説) and connection (接続) from h3 sections
  let description = "";
  let connection = "";
  const sectionHeadings = page.querySelectorAll("h2, h3, h4");
  for (const el of sectionHeadings) {
    const heading = el.text.trim();
    const next = el.nextElementSibling;
    if (!next || ["H2", "H3", "H4"].includes(next.tagName)) continue;
    const content = next.text.trim();

    if (heading.includes("解説") && !description) {
      description = content;
    } else if ((heading.includes("接続") || heading.includes("つなぎ方")) && !connection) {
      connection = content.slice(0, 200);
    }
  }

  // Extract examples from 例文 section
  const examples = extractMainichiExamples(page);

  const ruleName = toRuleNameFromUrl(url);

  return { ruleName, levels, name: grammarName, description, connection, pattern: "", examples };
}

/**
 * Extract structured example sentences from a mainichi-nonbiri page.
 * Returns [{ sentence, translations: [{ lang, text }] }]
 */
function extractMainichiExamples(page) {
  const examples = [];
  const headings = page.querySelectorAll("h2, h3, h4");

  for (const h of headings) {
    if (!h.text.includes("例文")) continue;

    let sibling = h.nextElementSibling;
    while (sibling && !["H2", "H3", "H4"].includes(sibling.tagName)) {
      const lines = sibling.text
        .split(/\n/)
        .map((l) => l.trim())
        .filter(Boolean);

      let i = 0;
      while (i < lines.length) {
        // Match numbered example: （1）or (10) or （１）followed by text
        const m = lines[i].match(
          /^[（(]\s*[\d０-９]+\s*[）)]\s*(.+?)(?:\s*▶?\s*)?$/,
        );
        if (m) {
          const sentence = m[1].replace(/\s*▶\s*$/, "").trim();
          const translations = [];

          let j = i + 1;
          while (j < lines.length) {
            if (/^[（(]\s*[\d０-９]+\s*[）)]/.test(lines[j])) break;
            const tl = lines[j].replace(/\s*▶\s*$/, "").trim();
            if (!tl || tl === "▶") {
              j++;
              continue;
            }
            // Chinese: has CJK ideographs but no hiragana/katakana
            if (
              /[\u4E00-\u9FFF]/.test(tl) &&
              !/[\u3040-\u309F\u30A0-\u30FF]/.test(tl)
            ) {
              translations.push({ lang: "zh", text: tl });
            } else if (/^[A-Za-z]/.test(tl)) {
              // English
              translations.push({ lang: "en", text: tl });
            }
            j++;
          }
          examples.push({ sentence, translations });
          i = j;
        } else {
          i++;
        }
      }
      sibling = sibling.nextElementSibling;
    }
    break; // Only process first 例文 section
  }

  return examples;
}

// ── Single URL mode ───────────────────────────────────────────────────────

async function scrapeSingleUrl(db, url) {
  const source = detectSource(url);
  console.log(`\nFetching: ${url}`);
  console.log(`Source type: ${source}`);

  const html = await fetchPage(url);
  if (!html) {
    console.error("Failed to fetch page");
    return;
  }

  let parsed;
  if (source === "mainichi_nonbiri") {
    parsed = parseMainichiPage(html, url);
  } else {
    console.error(`Unsupported source for single URL mode: ${source}`);
    console.error("Supported: mainichi-nonbiri URLs");
    return;
  }

  // Print parsed result
  console.log("\n── Parsed Data ──");
  console.log(`rule_name:   ${parsed.ruleName}`);
  console.log(`levels:      ${parsed.levels}`);
  console.log(`name:        ${parsed.name}`);
  console.log(`description: ${parsed.description.slice(0, 80)}...`);
  console.log(`connection:  ${parsed.connection.slice(0, 80)}${parsed.connection.length > 80 ? "..." : ""}`);
  console.log(`pattern:     ${parsed.pattern || "(empty)"}`);
  console.log(`examples:    ${parsed.examples.length}`);
  for (const [i, ex] of parsed.examples.entries()) {
    console.log(`  [${i + 1}] ${ex.sentence}`);
    for (const t of ex.translations) {
      console.log(`      ${t.lang}: ${t.text}`);
    }
  }

  if (dryRun) {
    console.log("\n(dry-run mode, not writing to database)");
    return;
  }

  // Write to database
  console.log("\n── Writing to DB ──");

  const grammarId = upsertGrammar(db, {
    ruleName: parsed.ruleName,
    levels: parsed.levels,
    name: parsed.name,
    description: parsed.description,
    connection: parsed.connection,
    pattern: parsed.pattern,
    source,
    sourceUrl: url,
    examples: parsed.examples,
  });
  console.log(`Upserted grammar: ${parsed.ruleName} (id=${grammarId})`);

  const exCount = db
    .prepare("SELECT COUNT(*) as c FROM examples WHERE grammar_id = ?")
    .get(grammarId).c;
  console.log(`Wrote ${exCount} example rows (${parsed.examples.length} sentences with translations)`);

  // Verify
  console.log("\n── Verification ──");
  const saved = db
    .prepare(
      "SELECT sentence, lang, sort_order FROM examples WHERE grammar_id = ? ORDER BY sort_order, lang",
    )
    .all(grammarId);
  for (const s of saved) {
    console.log(`  [${s.sort_order}] ${s.lang}: ${s.sentence.slice(0, 60)}`);
  }
}

// ── Scraper: mainichi-nonbiri.com (batch) ─────────────────────────────────

async function scrapeMainichiNonbiri(db) {
  const SOURCE = "mainichi_nonbiri";
  const INDEX_URL = "https://mainichi-nonbiri.com/japanese-grammar/";

  console.log(`\n[${SOURCE}] Fetching index page...`);
  const indexHtml = await fetchPage(INDEX_URL);
  if (!indexHtml) {
    console.error("  Failed to fetch index page");
    return;
  }

  const root = parseHTML(indexHtml);

  // Find grammar links
  const links = root
    .querySelectorAll("a[href]")
    .filter((a) => {
      const href = a.getAttribute("href") || "";
      return href.includes("/grammar/") || href.includes("grammar-");
    })
    .map((a) => ({
      url: a.getAttribute("href"),
      text: a.text.trim(),
    }))
    .filter((l) => l.text && l.url);

  // Deduplicate by URL
  const uniqueLinks = [...new Map(links.map((l) => [l.url, l])).values()];
  console.log(`  Found ${uniqueLinks.length} grammar links`);

  let count = 0;
  for (const link of uniqueLinks) {
    const url = link.url.startsWith("http")
      ? link.url
      : `https://mainichi-nonbiri.com${link.url}`;

    await sleep(1500);
    console.log(`  [${++count}/${uniqueLinks.length}] ${link.text}`);

    const html = await fetchPage(url);
    if (!html) continue;

    try {
      const parsed = parseMainichiPage(html, url);
      upsertGrammar(db, {
        ruleName: parsed.ruleName,
        levels: parsed.levels,
        name: parsed.name,
        description: parsed.description,
        connection: parsed.connection,
        pattern: parsed.pattern,
        source: SOURCE,
        sourceUrl: url,
        examples: parsed.examples,
      });
    } catch (e) {
      console.error(`  Error parsing ${url}: ${e.message}`);
    }
  }
}

// ── Scraper: nihongokyoshi-net.com ────────────────────────────────────────

async function scrapeNihongokyoshiNet(db) {
  const SOURCE = "nihongokyoshi_net";
  const INDEX_URL = "https://nihongokyoshi-net.com/jlpt-grammars/";

  console.log(`\n[${SOURCE}] Fetching index page...`);
  const indexHtml = await fetchPage(INDEX_URL);
  if (!indexHtml) {
    console.error("  Failed to fetch index page");
    return;
  }

  const root = parseHTML(indexHtml);

  const levelSections = root.querySelectorAll("h2, h3");
  const grammarLinks = [];

  for (const section of levelSections) {
    const levelMatch = section.text.match(/(N[1-5])/);
    if (!levelMatch) continue;
    const level = levelMatch[1];

    let sibling = section.nextElementSibling;
    while (sibling && !["H2", "H3"].includes(sibling.tagName)) {
      const links = sibling.querySelectorAll("a[href]");
      for (const a of links) {
        const href = a.getAttribute("href") || "";
        if (href.includes("grammar") || href.includes("jlpt")) {
          grammarLinks.push({ url: href, text: a.text.trim(), level });
        }
      }
      sibling = sibling.nextElementSibling;
    }
  }

  const uniqueLinks = [
    ...new Map(grammarLinks.map((l) => [l.url, l])).values(),
  ];
  console.log(`  Found ${uniqueLinks.length} grammar links`);

  let count = 0;
  for (const link of uniqueLinks) {
    const url = link.url.startsWith("http")
      ? link.url
      : `https://nihongokyoshi-net.com${link.url}`;

    await sleep(1500);
    console.log(
      `  [${++count}/${uniqueLinks.length}] ${link.text} (${link.level})`,
    );

    const html = await fetchPage(url);
    if (!html) continue;

    try {
      const page = parseHTML(html);

      const title =
        page.querySelector("h1")?.text?.trim() ||
        page.querySelector("title")?.text?.trim() ||
        link.text;
      const grammarName = title
        .replace(/\s*[|｜].*/g, "")
        .replace(/の意味.*/, "")
        .trim();

      const descEl = page.querySelector(
        ".entry-content p, article p, .post-body p",
      );
      const description = descEl ? descEl.text.trim().slice(0, 200) : "";

      let connection = "";
      const allElements = page.querySelectorAll("p, li, td, th, dt, dd");
      for (const el of allElements) {
        const text = el.text.trim();
        if (
          text.includes("接続") ||
          text.includes("つなぎ方") ||
          text.includes("Vて") ||
          text.includes("Nの")
        ) {
          const connMatch = text.match(/(?:接続|つなぎ方)[：:]?\s*(.+)/);
          if (connMatch) {
            connection = connMatch[1].trim().slice(0, 100);
            break;
          }
        }
      }

      const examples = [];
      const allP = page.querySelectorAll(
        ".example-sentence, .rei, .example, p, li",
      );
      for (const el of allP) {
        const text = el.text.trim();
        if (
          text &&
          /[\u3040-\u309F\u4E00-\u9FFF]/.test(text) &&
          text.length > 5 &&
          text.length < 150 &&
          !text.includes("接続") &&
          !text.includes("意味")
        ) {
          const cleaned = text
            .replace(/^[①②③④⑤⑥⑦⑧⑨⑩\d]+[.)）]\s*/, "")
            .trim();
          if (cleaned && !examples.includes(cleaned)) {
            examples.push(cleaned);
            if (examples.length >= 5) break;
          }
        }
      }

      upsertGrammar(db, {
        ruleName: toRuleNameFromUrl(url),
        levels: link.level,
        name: grammarName,
        description,
        connection,
        pattern: "",
        source: SOURCE,
        sourceUrl: url,
        examples, // string[] for nihongokyoshi
      });
    } catch (e) {
      console.error(`  Error parsing ${url}: ${e.message}`);
    }
  }
}

// ── Main ──────────────────────────────────────────────────────────────────

async function main() {
  const db = initDB();
  console.log(`Database: ${DB_PATH}`);

  // Single URL mode
  if (singleUrl) {
    await scrapeSingleUrl(db, singleUrl);
    db.close();
    return;
  }

  // Batch mode
  const sources = {
    "mainichi-nonbiri": scrapeMainichiNonbiri,
    "nihongokyoshi-net": scrapeNihongokyoshiNet,
  };

  if (sourceFilter) {
    const scraper = sources[sourceFilter];
    if (!scraper) {
      console.error(`Unknown source: ${sourceFilter}`);
      console.error(`Available sources: ${Object.keys(sources).join(", ")}`);
      process.exit(1);
    }
    await scraper(db);
  } else {
    for (const [name, scraper] of Object.entries(sources)) {
      console.log(`\n=== Scraping: ${name} ===`);
      await scraper(db);
    }
  }

  // Print summary
  const grammarCount = db
    .prepare("SELECT COUNT(*) as count FROM grammars")
    .get().count;
  const exampleCount = db
    .prepare("SELECT COUNT(*) as count FROM examples")
    .get().count;
  console.log(`\nDone! ${grammarCount} grammars, ${exampleCount} examples`);

  db.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
