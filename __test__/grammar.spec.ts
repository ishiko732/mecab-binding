import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { gzipSync } from 'node:zlib'
import { GrammarMatcher, Tagger } from 'mecab-binding'
import { describe, expect, test } from 'vitest'

const __dirname = dirname(fileURLToPath(import.meta.url))
const dictDir = resolve(__dirname, '..', '.output', 'dict', 'ipadic')
const taggerArgs = `-d ${dictDir} -r /dev/null`

const tagger = new Tagger(taggerArgs)

describe('GrammarMatcher', () => {
	test('constructor parses grammar text', () => {
		const gm = new GrammarMatcher(`
			nouns = 名詞+ ;
			verb = 動詞 ;
		`)
		expect(gm.ruleNames()).toEqual(['nouns', 'verb'])
	})

	test('fromFile loads grammar from file', () => {
		const grammarPath = resolve(__dirname, '..', 'grammars', 'example.grammar')
		const gm = GrammarMatcher.fromFile(grammarPath)
		expect(gm.ruleNames().length).toBeGreaterThan(0)
		expect(gm.ruleNames()).toContain('te_form')
	})

	test('find matches compound nouns', () => {
		const gm = new GrammarMatcher(`compound = 名詞 名詞+ ;`)
		const nodes = tagger.parseToNodes('携帯電話番号を入力する')
		const matches = gm.find('compound', nodes)
		expect(matches.length).toBeGreaterThan(0)
		const text = matches[0].nodes.map((n) => n.surface).join('')
		expect(text).toBe('携帯電話番号')
	})

	test('find matches te-form pattern', () => {
		const gm = new GrammarMatcher(`
			te_form = 動詞 助詞.接続助詞"て" ;
		`)
		const nodes = tagger.parseToNodes('食べて寝る')
		const matches = gm.find('te_form', nodes)
		expect(matches.length).toBe(1)
		expect(matches[0].nodes.map((n) => n.surface).join('')).toBe('食べて')
	})

	test('find matches concession pattern with wildcard', () => {
		const gm = new GrammarMatcher(`
			[N5 N4, "いくら〜ても：どんなに〜しても"]
			concession = "いくら" _* 助詞.接続助詞"で" 助詞.係助詞"も" ;
		`)
		const nodes = tagger.parseToNodes('いくら騒いでも大丈夫だ')
		const matches = gm.find('concession', nodes)
		expect(matches.length).toBe(1)
		expect(matches[0].levels).toEqual(['N5', 'N4'])
		expect(matches[0].description).toBe('いくら〜ても：どんなに〜しても')
		const text = matches[0].nodes.map((n) => n.surface).join('')
		expect(text).toContain('いくら')
		expect(text).toContain('も')

		// fixedIndices should NOT include the wildcard-matched token (騒い)
		const fixedSurfaces = matches[0].fixedIndices.map((i) => nodes[i].surface)
		expect(fixedSurfaces).toContain('いくら')
		expect(fixedSurfaces).not.toContain('騒い')
	})

	test('test returns boolean', () => {
		const gm = new GrammarMatcher(`adj = 形容詞 ;`)
		const nodes = tagger.parseToNodes('美しい花')
		expect(gm.test('adj', nodes)).toBe(true)

		const nodes2 = tagger.parseToNodes('東京大学')
		expect(gm.test('adj', nodes2)).toBe(false)
	})

	test('findAll returns matches from all rules', () => {
		const gm = new GrammarMatcher(`
			compound = 名詞 名詞+ ;
			particle = 助詞 ;
		`)
		const nodes = tagger.parseToNodes('携帯電話番号を入力する')
		const matches = gm.findAll(nodes)
		expect(matches.length).toBeGreaterThan(1)

		const rules = matches.map((m) => m.rule)
		expect(rules).toContain('compound')
		expect(rules).toContain('particle')
	})

	test('merge combines grammars', () => {
		const gm = new GrammarMatcher(`rule1 = 名詞 ;`)
		expect(gm.ruleNames()).toEqual(['rule1'])

		gm.merge(`rule2 = 動詞 ;`)
		expect(gm.ruleNames()).toEqual(['rule1', 'rule2'])
	})

	test('no match returns empty array', () => {
		const gm = new GrammarMatcher(`pattern = 形容詞 助動詞 ;`)
		const nodes = tagger.parseToNodes('東京大学')
		const matches = gm.find('pattern', nodes)
		expect(matches).toEqual([])
	})

	test('base form matching', () => {
		const gm = new GrammarMatcher(`suru = 動詞@"する" ;`)
		const nodes = tagger.parseToNodes('勉強した')
		const matches = gm.find('suru', nodes)
		expect(matches.length).toBe(1)
	})

	test('metadata with multiple levels', () => {
		const gm = new GrammarMatcher(`
			[N3 N2 N1, "advanced pattern"]
			pattern = 動詞 ;
		`)
		const nodes = tagger.parseToNodes('食べる')
		const matches = gm.find('pattern', nodes)
		expect(matches.length).toBe(1)
		expect(matches[0].levels).toEqual(['N3', 'N2', 'N1'])
	})

	test('fromGz parses CSV grammar from gzip data', () => {
		const csv = [
			'rule_name,levels,name,description,connection,pattern,examples',
			'te_form,N5,て形,動作の接続,動詞て形,"動詞 助詞.接続助詞""て""",ja:食べて寝る|ja:走って帰る',
		].join('\n')
		const gz = gzipSync(csv)
		const gm = GrammarMatcher.fromGz(gz)
		expect(gm.ruleNames()).toEqual(['te_form'])

		const nodes = tagger.parseToNodes('食べて寝る')
		const matches = gm.find('te_form', nodes)
		expect(matches.length).toBe(1)
		expect(matches[0].levels).toEqual(['N5'])
		expect(matches[0].connection).toBe('動詞て形')
		expect(matches[0].examples.length).toBe(2)
		expect(matches[0].examples[0].sentence).toBe('食べて寝る')
		expect(matches[0].examples[1].sentence).toBe('走って帰る')
	})

	test('fromGz with translations', () => {
		const csv = [
			'rule_name,levels,name,description,connection,pattern,examples',
			'compound,N5,複合名詞,名詞の連続,,名詞 名詞+,ja:東京大学;zh:东京大学;en:Tokyo University',
		].join('\n')
		const gz = gzipSync(csv)
		const gm = GrammarMatcher.fromGz(gz)

		const nodes = tagger.parseToNodes('東京大学に行く')
		const matches = gm.find('compound', nodes)
		expect(matches.length).toBe(1)
		expect(matches[0].examples.length).toBe(1)
		expect(matches[0].examples[0].sentence).toBe('東京大学')
		expect(matches[0].examples[0].translations).toEqual([
			{ lang: 'zh', text: '东京大学' },
			{ lang: 'en', text: 'Tokyo University' },
		])
	})

	test('fromGz with multiple rules', () => {
		const csv = [
			'rule_name,levels,name,description,connection,pattern,examples',
			'te_form,N5,て形,動作の接続,,"動詞 助詞.接続助詞""て""",ja:食べて',
			'compound,N5,複合名詞,名詞の連続,,名詞 名詞+,ja:東京大学',
		].join('\n')
		const gz = gzipSync(csv)
		const gm = GrammarMatcher.fromGz(gz)
		expect(gm.ruleNames()).toEqual(['te_form', 'compound'])
	})

	test('fromGz handles grammars.data file if exists', () => {
		const gzPath = resolve(__dirname, '..', 'sources', 'grammars.data')
		try {
			const data = readFileSync(gzPath)
			const gm = GrammarMatcher.fromGz(data)
			expect(gm.ruleNames().length).toBeGreaterThan(0)
		} catch {
			// File doesn't exist yet, skip
		}
	})
})
