import type { GrammarMatch, MecabNode } from 'mecab-binding'
import { useCallback, useMemo, useState } from 'react'
import { NodeCard } from './NodeCard'
import { useTagger } from './useTagger'

const DEFAULT_GRAMMAR = `// 譲歩
[N5 N4, "いくら〜ても：どんなに〜しても"]
ikura_temo = "いくら" _* 助詞.接続助詞"で" 助詞.係助詞"も" ;
`

const LEVEL_COLORS: Record<string, string> = {
	N5: '#4caf50',
	N4: '#2196f3',
	N3: '#ff9800',
	N2: '#e91e63',
	N1: '#9c27b0',
}

function App() {
	const [input, setInput] = useState('いくら騒いでも、ここは森の中の一軒家だから大丈夫だ')
	const [nodes, setNodes] = useState<MecabNode[]>([])
	const [grammarMatches, setGrammarMatches] = useState<GrammarMatch[]>([])
	const [parseError, setParseError] = useState<string | null>(null)
	const [showGrammarEditor, setShowGrammarEditor] = useState(false)
	const [grammarText, setGrammarText] = useState(DEFAULT_GRAMMAR)
	const { tagger, GrammarMatcher, gzMatcher, loading: taggerLoading, error: taggerError } = useTagger()

	const matcher = useMemo(() => {
		if (!GrammarMatcher) return null
		try {
			const base = gzMatcher ? gzMatcher.cloneMatcher() : new GrammarMatcher('')
			console.log('gzMatcher rules:', gzMatcher?.ruleNames().length ?? 0)
			if (grammarText.trim()) base.merge(grammarText)
			console.log('matcher rules:', base.ruleNames())
			return base
		} catch (e) {
			console.error('matcher build error:', e)
			return null
		}
	}, [grammarText, GrammarMatcher, gzMatcher])

	const handleParse = useCallback(() => {
		if (!tagger || !input.trim()) return
		setParseError(null)
		try {
			const result = tagger.parseToNodes(input)
			console.log('nodes:', result)
			setNodes(result)
			if (matcher) {
				const matches = matcher.findAll(result)
				console.log('grammarMatches:', matches)
				setGrammarMatches(matches)
			} else {
				setGrammarMatches([])
			}
		} catch (e) {
			console.error('parse error:', e)
			setParseError(e instanceof Error ? e.message : String(e))
		}
	}, [input, tagger, matcher])

	const error = taggerError || parseError

	// Filter out matches whose span is fully contained within a larger match's span.
	// This suppresses redundant sub-pattern cards (e.g. te_form inside n4_teiru).
	const displayedMatches = useMemo(() => {
		return grammarMatches.filter((m) => {
			const mLen = m.end - m.start
			return !grammarMatches.some(
				(other) =>
					other !== m &&
					other.start <= m.start &&
					other.end >= m.end &&
					other.end - other.start > mLen,
			)
		})
	}, [grammarMatches])

	// Build a map: node index -> { matches, isFixed }
	const nodeMatchMap = useMemo(() => {
		const map = new Map<number, { matches: GrammarMatch[]; isFixed: boolean }>()
		for (const m of grammarMatches) {
			const fixedSet = new Set(m.fixedIndices)
			for (let i = m.start; i < m.end; i++) {
				const existing = map.get(i)
				const isFixed = fixedSet.has(i)
				if (existing) {
					existing.matches.push(m)
					// If any match considers this node fixed, mark it as fixed
					if (isFixed) existing.isFixed = true
				} else {
					map.set(i, { matches: [m], isFixed })
				}
			}
		}
		return map
	}, [grammarMatches])

	return (
		<div style={{ maxWidth: 800, margin: '0 auto', padding: '2rem', fontFamily: 'system-ui, sans-serif' }}>
			<h1 style={{ fontSize: '1.5rem', marginBottom: '0.25rem' }}>mecab-binding Demo</h1>
			<p style={{ color: '#888', fontSize: '0.85rem', marginTop: 0, marginBottom: '1.5rem' }}>
				MeCab + WebAssembly Japanese morphological analyzer + Grammar pattern matcher
			</p>

			<div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
				<input
					type="text"
					value={input}
					onChange={(e) => setInput(e.target.value)}
					onKeyDown={(e) => e.key === 'Enter' && handleParse()}
					placeholder="日本語を入力..."
					style={{
						flex: 1,
						padding: '0.5rem 0.75rem',
						fontSize: '1rem',
						border: '1px solid #ccc',
						borderRadius: 6,
						outline: 'none',
					}}
				/>
				<button
					type="button"
					onClick={handleParse}
					disabled={taggerLoading || !tagger}
					style={{
						padding: '0.5rem 1.25rem',
						fontSize: '1rem',
						cursor: taggerLoading ? 'wait' : 'pointer',
						borderRadius: 6,
						border: 'none',
						background: '#0070f3',
						color: '#fff',
					}}
				>
					{taggerLoading ? '読込中...' : '解析'}
				</button>
			</div>

			<button
				type="button"
				onClick={() => setShowGrammarEditor(!showGrammarEditor)}
				style={{
					marginBottom: '1rem',
					padding: '0.3rem 0.75rem',
					fontSize: '0.8rem',
					cursor: 'pointer',
					borderRadius: 4,
					border: '1px solid #ddd',
					background: showGrammarEditor ? '#f0f0f0' : '#fff',
					color: '#555',
				}}
			>
				{showGrammarEditor ? '▼ Grammar Rules' : '▶ Grammar Rules'}
			</button>

			{showGrammarEditor && (
				<textarea
					value={grammarText}
					onChange={(e) => setGrammarText(e.target.value)}
					style={{
						width: '100%',
						height: 240,
						fontFamily: 'monospace',
						fontSize: '0.8rem',
						padding: '0.75rem',
						border: '1px solid #ccc',
						borderRadius: 6,
						marginBottom: '1rem',
						resize: 'vertical',
						lineHeight: 1.5,
						boxSizing: 'border-box',
					}}
				/>
			)}

			{error && (
				<div
					style={{
						padding: '0.75rem',
						background: '#fee',
						color: '#c00',
						borderRadius: 6,
						marginBottom: '1rem',
						fontSize: '0.85rem',
					}}
				>
					{error}
				</div>
			)}

			{grammarMatches.length > 0 && (
				<div style={{ marginBottom: '1rem' }}>
					<h3 style={{ fontSize: '0.9rem', color: '#555', marginBottom: '0.5rem' }}>Grammar Patterns Found</h3>
					<div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.5rem' }}>
						{displayedMatches.map((m, idx) => {
							const levelColor = m.levels[0] ? LEVEL_COLORS[m.levels[0]] || '#666' : '#666'
							return (
								<div
									key={`${m.rule}-${m.start}-${idx}`}
									style={{
										padding: '0.5rem 0.75rem',
										background: '#f8f9ff',
										border: `1px solid ${levelColor}33`,
										borderLeft: `3px solid ${levelColor}`,
										borderRadius: 6,
										fontSize: '0.8rem',
									}}
								>
									<div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', marginBottom: '0.25rem' }}>
										<span style={{ fontWeight: 600 }}>
											{m.nodes.map((n: MecabNode, ni: number) => {
												const globalIdx = m.start + ni
												const isFixed = m.fixedIndices.includes(globalIdx)
												return (
													<span key={globalIdx} style={{ opacity: isFixed ? 1 : 0.4 }}>
														{n.surface}
													</span>
												)
											})}
										</span>
										{m.levels.map((l: string) => (
											<span
												key={l}
												style={{
													fontSize: '0.65rem',
													padding: '0.1rem 0.35rem',
													borderRadius: 3,
													background: LEVEL_COLORS[l] || '#666',
													color: '#fff',
													fontWeight: 600,
												}}
											>
												{l}
											</span>
										))}
									</div>
									{m.description && <div style={{ color: '#777', fontSize: '0.75rem' }}>{m.description}</div>}
									{m.connection && (
										<div style={{ color: '#999', fontSize: '0.7rem', marginTop: '0.15rem' }}>
											接続: {m.connection}
										</div>
									)}
									{m.examples && m.examples.length > 0 && (
										<div style={{ marginTop: '0.25rem', paddingLeft: '0.5rem', borderLeft: '2px solid #e0e0e0' }}>
											{m.examples.slice(0, 3).map((ex) => (
												<div key={ex.sentence} style={{ fontSize: '0.7rem', color: '#888', lineHeight: 1.6 }}>
													{ex.sentence}
													{ex.translations && ex.translations.length > 0 && (
														<span style={{ color: '#aaa', marginLeft: '0.5rem' }}>
															({ex.translations.map((t: { text: string }) => t.text).join(' / ')})
														</span>
													)}
												</div>
											))}
										</div>
									)}
								</div>
							)
						})}
					</div>
				</div>
			)}

			{nodes.length > 0 && (
				<div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
					{nodes.map((node, idx) => (
						<NodeCard
						key={`${node.surface}-${node.id}`}
						node={node}
						grammarMatches={nodeMatchMap.get(idx)?.matches}
						isFixed={nodeMatchMap.get(idx)?.isFixed ?? false}
					/>
					))}
				</div>
			)}
		</div>
	)
}

export default App
