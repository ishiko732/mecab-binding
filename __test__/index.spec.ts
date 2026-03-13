import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { mecabVersion, Tagger } from 'mecab-binding'
import { expect, test } from 'vitest'

const require = createRequire(import.meta.url)
const __dirname = dirname(fileURLToPath(import.meta.url))
const dictDir = resolve(__dirname, '..', '.output', 'dict', 'ipadic')
const taggerArgs = `-d ${dictDir} -r /dev/null`

test('mecabVersion returns 0.996', () => {
	expect(mecabVersion()).toBe('0.996')
})

test('Tagger.parse returns correct result', () => {
	const tagger = new Tagger(taggerArgs)
	const result = tagger.parse('すもももももももものうち')
	expect(result).toContain('すもも')
	expect(result).toContain('もも')
	expect(result).toContain('EOS')
})

test('Tagger.parseToNodes returns nodes', () => {
	const tagger = new Tagger(taggerArgs)
	const nodes = tagger.parseToNodes('東京都に住んでいます')
	expect(nodes.length).toBeGreaterThan(0)

	const surfaces = nodes.map((n) => n.surface)
	expect(surfaces).toContain('東京')
	expect(surfaces).toContain('都')
	expect(surfaces).toContain('に')

	// Check node structure
	const tokyoNode = nodes.find((n) => n.surface === '東京')!
	expect(tokyoNode).toBeTruthy()
	expect(tokyoNode.feature).toBeTruthy()
	expect(tokyoNode.feature).toContain('名詞')
	expect(typeof tokyoNode.id).toBe('number')
	expect(typeof tokyoNode.cost).toBe('number')
	expect(typeof tokyoNode.wcost).toBe('number')
})

test('Tagger.parseNbest returns N-best results', () => {
	const tagger = new Tagger(taggerArgs)
	const result = tagger.parseNbest(3, '東京都に住んでいます')
	expect(result).toBeTruthy()
	const eosCount = (result.match(/EOS/g) || []).length
	expect(eosCount).toBeGreaterThanOrEqual(1)
})

test('Tagger N-best init/next', () => {
	const tagger = new Tagger(taggerArgs)
	tagger.parseNbestInit('東京都に住んでいます')
	const first = tagger.nextNbest()
	expect(first).toBeTruthy()
	expect(first!).toContain('EOS')

	const second = tagger.nextNbest()
	expect(second).toBeTruthy()
})

test('Tagger N-best nodes', () => {
	const tagger = new Tagger(taggerArgs)
	tagger.parseNbestInit('東京都に住んでいます')
	const nodes = tagger.nextNbestNodes()
	expect(nodes).toBeTruthy()
	expect(nodes!.length).toBeGreaterThan(0)
})

test('Tagger.dictionaryInfo returns dict info', () => {
	const tagger = new Tagger(taggerArgs)
	const info = tagger.dictionaryInfo()
	expect(info.length).toBeGreaterThan(0)
	expect(info[0].charset).toBe('utf-8')
	expect(info[0].size).toBeGreaterThan(0)
	expect(info[0].version).toBe(102)
})

test('Tagger partial and theta properties', () => {
	const tagger = new Tagger(taggerArgs)
	expect(tagger.partial).toBe(false)
	tagger.partial = true
	expect(tagger.partial).toBe(true)
	tagger.partial = false

	const theta = tagger.theta
	expect(typeof theta).toBe('number')
	tagger.theta = 0.5
	expect(Math.abs(tagger.theta - 0.5)).toBeLessThan(0.01)
})

test('Tagger constructor throws on invalid args', () => {
	expect(() => new Tagger('-d /nonexistent/path -r /dev/null')).toThrow()
})

test('parse Japanese text with various inputs', () => {
	const tagger = new Tagger(taggerArgs)

	// Simple hiragana
	const r1 = tagger.parseToNodes('こんにちは')
	expect(r1.length).toBeGreaterThan(0)

	// Katakana
	const r2 = tagger.parseToNodes('コンピューター')
	expect(r2.length).toBeGreaterThan(0)

	// Mixed
	const r3 = tagger.parseToNodes('私は学生です')
	expect(r3.length).toBeGreaterThan(0)
	const surfaces3 = r3.map((n) => n.surface)
	expect(surfaces3).toContain('私')
	expect(surfaces3).toContain('学生')

	// Empty string
	const r4 = tagger.parse('')
	expect(r4).toBe('EOS\n')
})

test('Tagger.fromBuffer creates tagger from .data file', () => {
	const ipadicPath = require.resolve('mecab-binding/ipadic.data')
	const buffer = readFileSync(ipadicPath)
	const tagger = Tagger.fromBuffer(buffer)

	// parse works
	const result = tagger.parse('すもももももももものうち')
	expect(result).toContain('すもも')
	expect(result).toContain('EOS')

	// parseToNodes works
	const nodes = tagger.parseToNodes('東京都に住んでいます')
	expect(nodes.length).toBeGreaterThan(0)
	const surfaces = nodes.map((n) => n.surface)
	expect(surfaces).toContain('東京')

	// dictionaryInfo works
	const info = tagger.dictionaryInfo()
	expect(info.length).toBeGreaterThan(0)
	expect(info[0].charset).toBe('utf-8')
})
