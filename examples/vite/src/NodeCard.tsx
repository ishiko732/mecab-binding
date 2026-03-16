import type { GrammarMatch, MecabNode } from 'mecab-binding'

const FEATURE_LABELS = ['品詞', '品詞細分類1', '品詞細分類2', '品詞細分類3', '活用型', '活用形', '原形', '読み', '発音'] as const

const LEVEL_COLORS: Record<string, string> = {
	N5: '#4caf50',
	N4: '#2196f3',
	N3: '#ff9800',
	N2: '#e91e63',
	N1: '#9c27b0',
}

function FeatureTag({ label, value }: { label: string; value: string }) {
	return (
		<span style={{ fontSize: '0.75rem', color: '#555' }}>
			<span style={{ color: '#999' }}>{label}:</span> {value}
		</span>
	)
}

export function NodeCard({
	node,
	grammarMatches,
	isFixed,
}: {
	node: MecabNode
	grammarMatches?: GrammarMatch[]
	isFixed: boolean
}) {
	const features = node.feature.split(',')
	const hasGrammar = grammarMatches && grammarMatches.length > 0
	const levelColor = hasGrammar && isFixed ? LEVEL_COLORS[grammarMatches[0].levels[0]] || '#0070f3' : '#e5e7eb'

	return (
		<div
			style={{
				border: `1px solid ${isFixed ? `${levelColor}66` : '#e5e7eb'}`,
				borderRadius: 8,
				padding: '0.75rem 1rem',
				background: isFixed ? `${levelColor}08` : '#fafafa',
				borderLeft: isFixed ? `3px solid ${levelColor}` : undefined,
			}}
		>
			<div style={{ display: 'flex', alignItems: 'baseline', gap: '0.75rem', marginBottom: '0.5rem' }}>
				<span style={{ fontSize: '1.25rem', fontWeight: 700 }}>{node.surface}</span>
				<span style={{ fontSize: '0.8rem', color: '#0070f3', fontWeight: 500 }}>{features[0]}</span>
				{features[6] && features[6] !== '*' && features[6] !== node.surface && (
					<span style={{ fontSize: '0.8rem', color: '#666' }}>{features[6]}</span>
				)}
				{features[7] && features[7] !== '*' && <span style={{ fontSize: '0.8rem', color: '#999' }}>{features[7]}</span>}
				{hasGrammar && isFixed && (
					<span style={{ marginLeft: 'auto', display: 'flex', gap: '0.3rem' }}>
						{grammarMatches.map((m) => (
							<span
								key={m.rule}
								style={{
									fontSize: '0.65rem',
									padding: '0.1rem 0.4rem',
									borderRadius: 3,
									background: LEVEL_COLORS[m.levels[0]] || '#666',
									color: '#fff',
									fontWeight: 500,
								}}
								title={m.description || m.rule}
							>
								{m.rule}
							</span>
						))}
					</span>
				)}
			</div>
			<div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.25rem 0.5rem' }}>
				{FEATURE_LABELS.map((label, j) => {
					const val = features[j]
					if (!val || val === '*') return null
					return <FeatureTag key={label} label={label} value={val} />
				})}
			</div>
		</div>
	)
}
