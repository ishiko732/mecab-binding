import type { Tagger } from 'mecab-binding'
import { initMecab } from 'mecab-binding/dynamic-wasi'
import ipadicUrl from 'mecab-binding/ipadic.data?url'
import wasiWorkerUrl from 'mecab-binding-wasm32-wasi/wasi-worker?url'
import wasmUrl from 'mecab-binding-wasm32-wasi/wasm?url'
import { useEffect, useMemo, useState } from 'react'

async function createTagger(): Promise<Tagger> {
	const { Tagger } = await initMecab({ wasm: wasmUrl, worker: wasiWorkerUrl })
	const res = await fetch(ipadicUrl)
	const buf = new Uint8Array(await res.arrayBuffer())
	return Tagger.fromBuffer(buf)
}

export function useTagger() {
	const [tagger, setTagger] = useState<Tagger | null>(null)
	const [loading, setLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)

	const promise = useMemo(() => createTagger(), [])

	useEffect(() => {
		let cancelled = false
		promise
			.then((t) => {
				if (!cancelled) setTagger(t)
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

	return { tagger, loading, error }
}
