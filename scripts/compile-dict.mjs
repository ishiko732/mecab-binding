import { execSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { dictIndex, packDict } from '../dist/index.js'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = resolve(__dirname, '..')

const IPADIC_TARBALL = resolve(root, 'sources', 'mecab-ipadic-2.7.0-20070610.tar.gz')
const inputDir = resolve(root, '.output', 'dict-src', 'ipadic')
const outputDir = resolve(root, '.output', 'dict', 'ipadic')

// Extract ipadic source from tarball if needed
if (!existsSync(inputDir)) {
	if (!existsSync(IPADIC_TARBALL)) {
		console.log('No mecab-ipadic-2.7.0-20070610.tar.gz found, skipping dictionary compilation')
		process.exit(0)
	}
	mkdirSync(inputDir, { recursive: true })
	execSync(`tar xzf "${IPADIC_TARBALL}" --strip-components=1 -C "${inputDir}"`, { stdio: 'inherit' })
}

if (existsSync(outputDir)) {
	console.log('Dictionary already compiled, skipping')
	process.exit(0)
}

console.log(`Compiling ipadic dictionary...`)
console.log(`  Input:  ${inputDir}`)
console.log(`  Output: ${outputDir}`)

mkdirSync(outputDir, { recursive: true })

try {
	dictIndex({
		inputDir,
		outputDir,
		fromCharset: 'euc-jp',
		toCharset: 'utf-8',
	})
	// Copy dicrc to output
	const dicrcSrc = resolve(inputDir, 'dicrc')
	if (existsSync(dicrcSrc)) {
		copyFileSync(dicrcSrc, resolve(outputDir, 'dicrc'))
	}
	console.log('Dictionary compiled successfully!')

	// Pack dictionary into single .data file
	const dataOutput = resolve(root, 'dist', 'ipadic.data')
	console.log(`Packing dictionary to ${dataOutput}...`)
	packDict(outputDir, dataOutput)
	console.log('Dictionary packed successfully!')
} catch (err) {
	console.error('Failed to compile dictionary:', err.message)
	process.exit(1)
}
