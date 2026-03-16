#!/usr/bin/env node

/**
 * Compiles grammar data from SQLite into a gzip-compressed CSV file.
 *
 * Usage:
 *   node scripts/compile-grammar.mjs
 *
 * Output: sources/grammars.data
 */

import { createWriteStream, existsSync, statSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { Readable } from 'node:stream'
import { pipeline } from 'node:stream/promises'
import { fileURLToPath } from 'node:url'
import { createGzip } from 'node:zlib'
import Database from 'better-sqlite3'

const __dirname = dirname(fileURLToPath(import.meta.url))
const DB_PATH = resolve(__dirname, '..', 'sources', 'grammars.sqlite')
const OUTPUT_PATH = resolve(__dirname, '..', 'sources', 'grammars.data')

// ── CSV helpers ───────────────────────────────────────────────────────────

/** Escape a CSV field (double quotes if needed) */
function csvEscape(value) {
	if (!value) return ''
	// Normalize newlines to spaces to keep each grammar record on a single CSV line
	const str = String(value)
		.replace(/\r?\n\s*/g, ' ')
		.trim()
	if (str.includes(',') || str.includes('"')) {
		return `"${str.replace(/"/g, '""')}"`
	}
	return str
}

// ── Main ──────────────────────────────────────────────────────────────────

function main() {
	if (!existsSync(DB_PATH)) {
		console.error(`Database not found: ${DB_PATH}`)
		console.error("Run 'node scripts/scrape-grammar.mjs' first.")
		process.exit(1)
	}

	const db = new Database(DB_PATH, { readonly: true })

	// Fetch all grammars
	const grammars = db
		.prepare(
			`SELECT id, rule_name, levels, name, description, connection, pattern
       FROM grammars
       ORDER BY id`,
		)
		.all()

	// Fetch all examples grouped by grammar_id
	const examplesStmt = db.prepare(
		`SELECT sentence, lang, sort_order
     FROM examples
     WHERE grammar_id = ?
     ORDER BY sort_order`,
	)

	// Build CSV content
	const header = 'rule_name,levels,name,description,connection,pattern,examples'
	const lines = [header]

	for (const g of grammars) {
		const examples = examplesStmt.all(g.id)

		// Format examples: "ja:text1;zh:翻译|ja:text2;zh:翻译2"
		// Group by sort_order: same sort_order joined with ";", different groups with "|"
		const groups = new Map()
		for (const ex of examples) {
			if (!groups.has(ex.sort_order)) groups.set(ex.sort_order, [])
			groups.get(ex.sort_order).push(`${ex.lang}:${ex.sentence}`)
		}
		const examplesStr = [...groups.values()].map((parts) => parts.join(';')).join('|')

		const row = [
			csvEscape(g.rule_name),
			csvEscape(g.levels),
			csvEscape(g.name),
			csvEscape(g.description),
			csvEscape(g.connection),
			csvEscape(g.pattern),
			csvEscape(examplesStr),
		].join(',')

		lines.push(row)
	}

	db.close()

	const csvContent = `${lines.join('\n')}\n`

	// Write gzipped output
	const gzip = createGzip({ level: 9 })
	const output = createWriteStream(OUTPUT_PATH)

	pipeline(Readable.from(csvContent), gzip, output).then(() => {
		const stats = statSync(OUTPUT_PATH)
		console.log(`Written: ${OUTPUT_PATH}`)
		console.log(`  ${grammars.length} grammars, ${csvContent.length} bytes CSV -> ${stats.size} bytes gzipped`)
	})
}

main()
