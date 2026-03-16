import type { GrammarMatcher, Tagger } from 'mecab-binding'
import { initMecab } from 'mecab-binding/dynamic-wasi'
import grammarsUrl from 'mecab-binding/grammars.data?url'
import ipadicUrl from 'mecab-binding/ipadic.data?url'
import wasiWorkerUrl from 'mecab-binding-wasm32-wasi/wasi-worker?url'
import wasmUrl from 'mecab-binding-wasm32-wasi/wasm?url'
import { useEffect, useMemo, useState } from 'react'

interface MecabBinding {
	tagger: Tagger
	GrammarMatcher: typeof GrammarMatcher
	gzMatcher: GrammarMatcher | null
}

async function createBinding(): Promise<MecabBinding> {
	const { Tagger, GrammarMatcher } = await initMecab({ wasm: wasmUrl, worker: wasiWorkerUrl })
	const res = await fetch(ipadicUrl)
	const buf = new Uint8Array(await res.arrayBuffer())
	const tagger = Tagger.fromBuffer(buf)

	let gzMatcher: GrammarMatcher | null = null
	try {
		const gzRes = await fetch(grammarsUrl)
		if (gzRes.ok) {
			const buf = new Uint8Array(await gzRes.arrayBuffer())
			gzMatcher = GrammarMatcher.fromGz(buf)
		}
	} catch {
		// grammars.data not available, skip
	}

	return { tagger, GrammarMatcher, gzMatcher }
}

export function useTagger() {
	const [tagger, setTagger] = useState<Tagger | null>(null)
	const [GrammarMatcherClass, setGrammarMatcherClass] = useState<typeof GrammarMatcher | null>(null)
	const [gzMatcher, setGzMatcher] = useState<GrammarMatcher | null>(null)
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
					setGzMatcher(b.gzMatcher)
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

	return { tagger, GrammarMatcher: GrammarMatcherClass, gzMatcher, loading, error }
}
