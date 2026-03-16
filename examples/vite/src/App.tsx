import type { MecabNode } from 'mecab-binding'
import { useCallback, useState } from 'react'
import { NodeCard } from './NodeCard'
import { useTagger } from './useTagger'

function App() {
	const [input, setInput] = useState('すもももももももものうち')
	const [nodes, setNodes] = useState<MecabNode[]>([])
	const [parseError, setParseError] = useState<string | null>(null)
	const { tagger, loading: taggerLoading, error: taggerError } = useTagger()

	const handleParse = useCallback(() => {
		if (!tagger || !input.trim()) return
		setParseError(null)
		try {
			const result = tagger.parseToNodes(input)
			setNodes(result)
		} catch (e) {
			console.error('parse error:', e)
			setParseError(e instanceof Error ? e.message : String(e))
		}
	}, [input, tagger])

	const error = taggerError || parseError

	return (
		<div style={{ maxWidth: 720, margin: '0 auto', padding: '2rem', fontFamily: 'system-ui, sans-serif' }}>
			<h1 style={{ fontSize: '1.5rem', marginBottom: '0.25rem' }}>mecab-binding Demo</h1>
			<p style={{ color: '#888', fontSize: '0.85rem', marginTop: 0, marginBottom: '1.5rem' }}>
				MeCab + WebAssembly Japanese morphological analyzer
			</p>

			<div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1.5rem' }}>
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

			{nodes.length > 0 && (
				<div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
					{nodes.map((node) => (
						<NodeCard key={`${node.surface}-${node.id}`} node={node} />
					))}
				</div>
			)}
		</div>
	)
}

export default App
