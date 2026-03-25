#!/usr/bin/env node

/**
 * Helper script for sub-agents to read and update grammar patterns.
 *
 * Usage:
 *   node scripts/update-patterns.mjs read <offset> <limit>
 *   node scripts/update-patterns.mjs write <id> <pattern>
 */

import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import Database from 'better-sqlite3'

const __dirname = dirname(fileURLToPath(import.meta.url))
const DB_PATH = resolve(__dirname, '..', 'sources', 'grammars.sqlite')

const [, , cmd, ...args] = process.argv

const db = new Database(DB_PATH)

if (cmd === 'read') {
	const offset = parseInt(args[0]) || 0
	const limit = parseInt(args[1]) || 50
	const rows = db
		.prepare('SELECT id, rule_name, connection FROM grammars ORDER BY id LIMIT ? OFFSET ?')
		.all(limit, offset)
	console.log(JSON.stringify(rows, null, 2))
} else if (cmd === 'write') {
	const id = parseInt(args[0])
	const pattern = args[1] || ''
	db.prepare('UPDATE grammars SET pattern = ? WHERE id = ?').run(pattern, id)
	console.log(`Updated id=${id}`)
} else if (cmd === 'write-batch') {
	// Read JSON array of {id, pattern} from stdin
	let input = ''
	process.stdin.on('data', (d) => (input += d))
	process.stdin.on('end', () => {
		const updates = JSON.parse(input)
		const stmt = db.prepare('UPDATE grammars SET pattern = ? WHERE id = ?')
		const runAll = db.transaction(() => {
			for (const { id, pattern } of updates) {
				stmt.run(pattern || '', id)
			}
		})
		runAll()
		console.log(`Updated ${updates.length} records`)
	})
} else {
	console.error('Usage: update-patterns.mjs read <offset> <limit>')
	console.error('       update-patterns.mjs write <id> <pattern>')
	console.error('       update-patterns.mjs write-batch  (JSON array from stdin)')
	process.exit(1)
}
