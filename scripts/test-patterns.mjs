#!/usr/bin/env node
/**
 * Test grammar patterns against their Japanese example sentences from the DB.
 *
 * For each rule with examples, parses each ja sentence with MeCab and checks
 * whether the rule matches. Reports failures so patterns can be fixed.
 *
 * Usage:
 *   node scripts/test-patterns.mjs                     # test all rules
 *   node scripts/test-patterns.mjs --rule=n4_teiru     # test one rule
 *   node scripts/test-patterns.mjs --level=N4          # test all N4 rules
 *   node scripts/test-patterns.mjs --search=ばかり      # filter by text in rule_name or pattern
 *   node scripts/test-patterns.mjs --fail-only         # suppress passing rows
 *   node scripts/test-patterns.mjs --nodes             # show MeCab tokens on failure
 *   node scripts/test-patterns.mjs --limit=10          # stop after 10 failures
 *
 * Requires:
 *   pnpm build              (native .node binding in dist/)
 *   sources/grammars.sqlite (populated by scrape-grammar.mjs)
 *   .output/dict/ipadic     (built by compile-dict.mjs)
 */

import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import Database from 'better-sqlite3'

const require = createRequire(import.meta.url)
const __dirname = dirname(fileURLToPath(import.meta.url))
const ROOT = resolve(__dirname, '..')

// ── Args ─────────────────────────────────────────────────────────────────────

const args = process.argv.slice(2)
const ruleFlag = args.find((a) => a.startsWith('--rule='))
const levelFlag = args.find((a) => a.startsWith('--level='))
const searchFlag = args.find((a) => a.startsWith('--search='))
const limitFlag = args.find((a) => a.startsWith('--limit='))
const failOnly = args.includes('--fail-only')
const showNodes = args.includes('--nodes')

const ruleFilter = ruleFlag ? ruleFlag.split('=')[1] : null
const levelFilter = levelFlag ? levelFlag.split('=')[1] : null
const searchText = searchFlag ? searchFlag.split('=').slice(1).join('=') : null
const failLimit = limitFlag ? parseInt(limitFlag.split('=')[1], 10) : Infinity

// ── Load native binding ───────────────────────────────────────────────────────

const bindingPath = resolve(ROOT, 'dist/index.js')
if (!existsSync(bindingPath)) {
  console.error(`Binding not found: ${bindingPath}`)
  console.error("Run 'pnpm build' first.")
  process.exit(1)
}

const { Tagger, GrammarMatcher } = require(bindingPath)

const dictDir = resolve(ROOT, '.output/dict/ipadic')
if (!existsSync(dictDir)) {
  console.error(`Dict not found: ${dictDir}`)
  console.error("Run 'node scripts/compile-dict.mjs' first.")
  process.exit(1)
}

const tagger = new Tagger(`-d ${dictDir} -r /dev/null`)

// ── Load DB ───────────────────────────────────────────────────────────────────

const dbPath = resolve(ROOT, 'sources', 'grammars.sqlite')
if (!existsSync(dbPath)) {
  console.error(`DB not found: ${dbPath}`)
  console.error("Run 'node scripts/scrape-grammar.mjs' first.")
  process.exit(1)
}

const db = new Database(dbPath, { readonly: true })
const allGrammars = db
  .prepare('SELECT id, rule_name, levels, pattern FROM grammars WHERE pattern IS NOT NULL ORDER BY id')
  .all()

// ── Build a single matcher with ALL rules ─────────────────────────────────────
// Loading all rules together allows patterns that reference other rules to work.

const grammarLines = []
for (const g of allGrammars) {
  if (!g.pattern?.trim()) continue
  grammarLines.push(`${g.rule_name} = ${g.pattern} ;`)
}

let matcher
try {
  matcher = new GrammarMatcher(grammarLines.join('\n'))
} catch (e) {
  console.error('Failed to build matcher:', e.message)
  process.exit(1)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function formatNodes(nodes) {
  return nodes
    .map((n, i) => {
      const parts = n.feature.split(',')
      const pos = parts.slice(0, 4).filter((p) => p !== '*').join(',')
      const conj = parts[5] !== '*' ? ` 活用形=${parts[5]}` : ''
      const base = parts[6] !== '*' && parts[6] !== n.surface ? ` base=${parts[6]}` : ''
      return `    [${i}] ${n.surface.padEnd(8)} ${pos}${conj}${base}`
    })
    .join('\n')
}

// ── Filter rules to test ──────────────────────────────────────────────────────

const rulesToTest = allGrammars.filter((g) => {
  if (!g.pattern?.trim()) return false
  if (ruleFilter && g.rule_name !== ruleFilter) return false
  if (levelFilter) {
    const ruleLevels = (g.levels || '').split(',').map((l) => l.trim())
    if (!ruleLevels.includes(levelFilter)) return false
  }
  if (searchText) {
    const hay = `${g.rule_name} ${g.pattern}`
    if (!hay.includes(searchText)) return false
  }
  return true
})

const examplesStmt = db.prepare(
  `SELECT sentence FROM examples WHERE grammar_id = ? AND lang = 'ja' ORDER BY sort_order`,
)

const excludedSet = new Set(
  db.prepare('SELECT grammar_id, sentence FROM excluded_examples').all()
    .map((e) => `${e.grammar_id}:${e.sentence.trim()}`),
)

// ── Run tests ─────────────────────────────────────────────────────────────────

let total = 0
let passed = 0
let failed = 0
let skipped = 0 // no ja examples

const PASS = '\x1b[32m✓\x1b[0m'
const FAIL = '\x1b[31m✗\x1b[0m'

for (const g of rulesToTest) {
  const examples = examplesStmt.all(g.id).map((e) => e.sentence.trim()).filter(
    (s) => s && !excludedSet.has(`${g.id}:${s}`),
  )

  if (examples.length === 0) {
    skipped++
    continue
  }

  total++
  const levelLabel = (g.levels || '').padEnd(6)
  const failures = []

  for (const sentence of examples) {
    try {
      const nodes = tagger.parseToNodes(sentence).filter((n) => n.stat !== 2 && n.stat !== 3)
      const hits = matcher.find(g.rule_name, nodes)
      if (hits.length === 0) {
        failures.push({ sentence, nodes })
      }
    } catch (e) {
      failures.push({ sentence, nodes: [], error: e.message })
    }
  }

  if (failures.length === 0) {
    passed++
    if (!failOnly) {
      console.log(`${PASS} ${levelLabel} ${g.rule_name}`)
    }
  } else {
    failed++
    console.log(`${FAIL} ${levelLabel} ${g.rule_name}`)
    console.log(`      pattern: ${g.pattern}`)
    for (const f of failures) {
      if (f.error) {
        console.log(`      ✗ ERROR: ${f.sentence}`)
        console.log(`        ${f.error}`)
      } else {
        console.log(`      ✗ ${f.sentence}`)
        if (showNodes) {
          console.log(formatNodes(f.nodes))
        }
      }
    }
    if (failed >= failLimit) {
      console.log(`\nReached failure limit (${failLimit}), stopping early.`)
      break
    }
  }
}

db.close()

// ── Summary ───────────────────────────────────────────────────────────────────

console.log()
console.log(
  `Tested: ${total}  ` +
    `\x1b[32mPassed: ${passed}\x1b[0m  ` +
    `\x1b[31mFailed: ${failed}\x1b[0m  ` +
    `Skipped (no ja examples): ${skipped}`,
)
