import type { MecabNode } from 'mecab-binding'

const FEATURE_LABELS = ['品詞', '品詞細分類1', '品詞細分類2', '品詞細分類3', '活用型', '活用形', '原形', '読み', '発音'] as const

function FeatureTag({ label, value }: { label: string; value: string }) {
	return (
		<span style={{ fontSize: '0.75rem', color: '#555' }}>
			<span style={{ color: '#999' }}>{label}:</span> {value}
		</span>
	)
}

export function NodeCard({ node }: { node: MecabNode }) {
	const features = node.feature.split(',')
	return (
		<div
			style={{
				border: '1px solid #e5e7eb',
				borderRadius: 8,
				padding: '0.75rem 1rem',
				background: '#fafafa',
			}}
		>
			<div style={{ display: 'flex', alignItems: 'baseline', gap: '0.75rem', marginBottom: '0.5rem' }}>
				<span style={{ fontSize: '1.25rem', fontWeight: 700 }}>{node.surface}</span>
				<span style={{ fontSize: '0.8rem', color: '#0070f3', fontWeight: 500 }}>{features[0]}</span>
				{features[6] && features[6] !== '*' && features[6] !== node.surface && (
					<span style={{ fontSize: '0.8rem', color: '#666' }}>{features[6]}</span>
				)}
				{features[7] && features[7] !== '*' && <span style={{ fontSize: '0.8rem', color: '#999' }}>{features[7]}</span>}
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
