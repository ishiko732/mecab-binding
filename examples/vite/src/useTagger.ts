import type { GrammarMatcher, Tagger } from 'mecab-binding'
import { initMecab } from 'mecab-binding/dynamic-wasi'
import ipadicUrl from 'mecab-binding/ipadic.data?url'
import wasiWorkerUrl from 'mecab-binding-wasm32-wasi/wasi-worker?url'
import wasmUrl from 'mecab-binding-wasm32-wasi/wasm?url'
import { useEffect, useMemo, useState } from 'react'

interface MecabBinding {
	tagger: Tagger
	GrammarMatcher: typeof GrammarMatcher
}

async function createBinding(): Promise<MecabBinding> {
	const binding = await initMecab({ wasm: wasmUrl, worker: wasiWorkerUrl })
	const res = await fetch(ipadicUrl)
	const buf = new Uint8Array(await res.arrayBuffer())
	const tagger = binding.Tagger.fromBuffer(buf)
	return { tagger, GrammarMatcher: binding.GrammarMatcher }
}

export function useTagger() {
	const [tagger, setTagger] = useState<Tagger | null>(null)
	const [GrammarMatcherClass, setGrammarMatcherClass] = useState<typeof GrammarMatcher | null>(null)
	const [loading, setLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)

	const promise = useMemo(() => createBinding(), [])

	useEffect(() => {
		let cancelled = false
		promise
			.then((b) => {
				if (!cancelled) {
					setTagger(b.tagger)
					setGrammarMatcherClass(() => b.GrammarMatcher)
				}
			})
			.catch((e) => {
				if (!cancelled) setError(e instanceof Error ? e.message : String(e))
			})
			.finally(() => {
				if (!cancelled) setLoading(false)
			})
		return () => {
			cancelled = true
		}
	}, [promise])

	return { tagger, GrammarMatcher: GrammarMatcherClass, loading, error }
}
