import { useState, useRef, useCallback } from 'react'
import type { Tagger, MecabNode } from 'mecab-binding'
import ipadicUrl from 'mecab-binding/ipadic.data?url'

const FEATURE_LABELS = [
  '品詞',
  '品詞細分類1',
  '品詞細分類2',
  '品詞細分類3',
  '活用型',
  '活用形',
  '原形',
  '読み',
  '発音',
] as const

function App() {
  const [input, setInput] = useState('すもももももももものうち')
  const [nodes, setNodes] = useState<MecabNode[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const taggerRef = useRef<Tagger | null>(null)

  const ensureTagger = useCallback(async () => {
    if (taggerRef.current) return taggerRef.current
    const { Tagger } = await import('mecab-binding')
    const res = await fetch(ipadicUrl)
    const buf = new Uint8Array(await res.arrayBuffer())
    const tagger = Tagger.fromBuffer(buf)
    taggerRef.current = tagger
    return tagger
  }, [])

  const handleParse = useCallback(async () => {
    if (!input.trim()) return
    setLoading(true)
    setError(null)
    try {
      const tagger = await ensureTagger()
      const result = tagger.parseToNodes(input)
      // Filter out BOS/EOS nodes (stat >= 2)
      setNodes(result.filter((n) => n.stat < 2))
    } catch (e) {
      console.error('parse error:', e)
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setLoading(false)
    }
  }, [input, ensureTagger])

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
          onClick={handleParse}
          disabled={loading}
          style={{
            padding: '0.5rem 1.25rem',
            fontSize: '1rem',
            cursor: loading ? 'wait' : 'pointer',
            borderRadius: 6,
            border: 'none',
            background: '#0070f3',
            color: '#fff',
          }}
        >
          {loading ? '読込中...' : '解析'}
        </button>
      </div>

      {error && (
        <div style={{ padding: '0.75rem', background: '#fee', color: '#c00', borderRadius: 6, marginBottom: '1rem', fontSize: '0.85rem' }}>
          {error}
        </div>
      )}

      {nodes.length > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
          {nodes.map((node, i) => {
            const features = node.feature.split(',')
            return (
              <div key={i} style={{
                border: '1px solid #e5e7eb',
                borderRadius: 8,
                padding: '0.75rem 1rem',
                background: '#fafafa',
              }}>
                <div style={{ display: 'flex', alignItems: 'baseline', gap: '0.75rem', marginBottom: '0.5rem' }}>
                  <span style={{ fontSize: '1.25rem', fontWeight: 700 }}>{node.surface}</span>
                  <span style={{ fontSize: '0.8rem', color: '#0070f3', fontWeight: 500 }}>{features[0]}</span>
                  {features[6] && features[6] !== '*' && features[6] !== node.surface && (
                    <span style={{ fontSize: '0.8rem', color: '#666' }}>{features[6]}</span>
                  )}
                  {features[7] && features[7] !== '*' && (
                    <span style={{ fontSize: '0.8rem', color: '#999' }}>{features[7]}</span>
                  )}
                </div>
                <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.25rem 0.5rem' }}>
                  {FEATURE_LABELS.map((label, j) => {
                    const val = features[j]
                    if (!val || val === '*') return null
                    return (
                      <span key={j} style={{ fontSize: '0.75rem', color: '#555' }}>
                        <span style={{ color: '#999' }}>{label}:</span> {val}
                      </span>
                    )
                  })}
                </div>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}

export default App
