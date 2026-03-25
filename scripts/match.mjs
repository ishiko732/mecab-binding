#!/usr/bin/env node
/**
 * Grammar match CLI — quickly test grammar matching against Japanese text.
 *
 * Usage:
 *   node scripts/match.mjs "食べています"
 *   node scripts/match.mjs "食べています" --nodes
 *   node scripts/match.mjs "食べています" --rule n4_teiru
 *   node scripts/match.mjs "食べています" --rules n4_teiru,te_form
 *
 * Requires native build: pnpm build
 * Requires compiled grammar: node scripts/compile-grammar.mjs
 */

import { createRequire } from 'node:module'
import { readFileSync } from 'node:fs'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const require = createRequire(import.meta.url)
const __dirname = dirname(fileURLToPath(import.meta.url))
const ROOT = resolve(__dirname, '..')

// ── Args ────────────────────────────────────────────────────────────────────

const args = process.argv.slice(2)
const sentence = args.find((a) => !a.startsWith('--'))
const showNodes = args.includes('--nodes')
const ruleFlag = args.find((a) => a.startsWith('--rule=') || a.startsWith('--rules='))
const ruleFilter = ruleFlag ? ruleFlag.split('=')[1].split(',') : null

if (!sentence) {
  console.error('Usage: node scripts/match.mjs <sentence> [--nodes] [--rule=<name>]')
  process.exit(1)
}

// ── Load binding ─────────────────────────────────────────────────────────────

const { Tagger, GrammarMatcher } = require(resolve(ROOT, 'dist/index.js'))

const tagger = new Tagger(`-d ${resolve(ROOT, '.output/dict/ipadic')} -r /dev/null`)
const gzData = new Uint8Array(readFileSync(resolve(ROOT, 'dist/grammars.data')))
const matcher = GrammarMatcher.fromGz(gzData)

// ── Parse ────────────────────────────────────────────────────────────────────

const nodes = tagger.parseToNodes(sentence).filter((n) => n.stat !== 2 && n.stat !== 3)

// ── Show nodes ───────────────────────────────────────────────────────────────

if (showNodes) {
  console.log('\n── Nodes ──────────────────────────────────────────')
  nodes.forEach((n, i) => {
    const parts = n.feature.split(',')
    const pos = parts.slice(0, 4).filter((p) => p !== '*').join(',')
    const conj = parts[5] !== '*' ? ` 活用形=${parts[5]}` : ''
    const base = parts[6] !== '*' && parts[6] !== n.surface ? ` base=${parts[6]}` : ''
    console.log(`  [${i}] ${n.surface.padEnd(8)} ${pos}${conj}${base}`)
  })
}

// ── Match ────────────────────────────────────────────────────────────────────

const allMatches = matcher.findAll(nodes)
const matches = ruleFilter
  ? allMatches.filter((m) => ruleFilter.includes(m.rule))
  : allMatches

console.log('\n── Matches ────────────────────────────────────────')
if (matches.length === 0) {
  console.log('  (no matches)')
} else {
  for (const m of matches) {
    const fixedSurfaces = m.fixedIndices.map((i) => nodes[i].surface).join(' ')
    const wildcardSurfaces = m.nodes
      .map((n, localIdx) => {
        const globalIdx = m.start + localIdx
        return m.fixedIndices.includes(globalIdx) ? `[${n.surface}]` : n.surface
      })
      .join(' ')
    const levels = m.levels.length ? m.levels.join('/') : 'N?'
    const desc = m.description ? m.description.split('：')[0] : ''
    console.log(`  ${levels.padEnd(6)} ${m.rule.padEnd(30)} fixed: ${fixedSurfaces}`)
    if (desc) console.log(`         ${desc}`)
    console.log(`         span: ${wildcardSurfaces}`)
  }
}
console.log()
