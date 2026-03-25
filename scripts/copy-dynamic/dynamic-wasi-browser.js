import {
  getDefaultContext as __emnapiGetDefaultContext,
  instantiateNapiModuleSync as __emnapiInstantiateNapiModuleSync,
  WASI as __WASI,
  createOnMessage as __wasmCreateOnMessageForFsProxy,
} from '@napi-rs/wasm-runtime'
import { memfs } from '@napi-rs/wasm-runtime/fs'

export const { fs: __fs, vol: __volume } = memfs()
/**
 * Initialize the MeCab NAPI module with the given WASM binary and worker URL.
 *
 * @param {object} options
 * @param {BufferSource | WebAssembly.Module} options.wasm - The WASM binary (ArrayBuffer, TypedArray, or compiled Module).
 * @param {URL | string} [options.worker] - URL to the WASI worker script. If omitted, async work pool is disabled.
 * @returns {Promise<typeof import('./index')>}
 */
export async function initMecab(options) {
	const { wasm, worker } = options
	const __wasi = new __WASI({
		version: 'preview1',
		fs: __fs,
		preopens: {
			'/': '/',
		},
	})

	const __emnapiContext = __emnapiGetDefaultContext()

	const __sharedMemory = new WebAssembly.Memory({
		initial: 4000,
		maximum: 65536,
		shared: true,
	})

	let wasmBinary
	if (wasm instanceof WebAssembly.Module) {
		wasmBinary = wasm
	} else if (wasm instanceof ArrayBuffer || ArrayBuffer.isView(wasm)) {
		wasmBinary = wasm
	} else if (typeof wasm === 'string' || wasm instanceof URL) {
		wasmBinary = await fetch(String(wasm)).then((res) => res.arrayBuffer())
	} else {
		throw new TypeError('options.wasm must be a BufferSource, WebAssembly.Module, URL, or string')
	}

	const { napiModule: __napiModule } = __emnapiInstantiateNapiModuleSync(wasmBinary, {
		context: __emnapiContext,
		asyncWorkPoolSize: worker ? 4 : 0,
		wasi: __wasi,
		onCreateWorker() {
			if (!worker) {
				throw new Error('options.worker is required to use async work pool')
			}
			const workerInstance = new Worker(worker instanceof URL ? worker : new URL(worker, import.meta.url), {
				type: 'module',
			})
			worker.addEventListener('message', __wasmCreateOnMessageForFsProxy(__fs))
			worker.addEventListener('error', (event) => {
				if (event.data && typeof event.data === 'object' && event.data.type === 'error') {
					window.dispatchEvent(new CustomEvent('napi-rs-worker-error', { detail: event.data }))
				}
			})
      
			return workerInstance
		},
		overwriteImports(importObject) {
			importObject.env = {
				...importObject.env,
				...importObject.napi,
				...importObject.emnapi,
				memory: __sharedMemory,
			}
			return importObject
		},
		beforeInit({ instance }) {
			for (const name of Object.keys(instance.exports)) {
				if (name.startsWith('__napi_register__')) {
					instance.exports[name]()
				}
			}
		},
	})

	return __napiModule.exports
}
