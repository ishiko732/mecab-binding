import {
  getDefaultContext as __emnapiGetDefaultContext,
  instantiateNapiModuleSync as __emnapiInstantiateNapiModuleSync,
  WASI as __WASI,
  createOnMessage as __wasmCreateOnMessageForFsProxy,
} from '@napi-rs/wasm-runtime'
import { memfs as __memfs } from '@napi-rs/wasm-runtime/fs'
import __wasmUrl from 'mecab-binding/mecab.wasm?url'

export const { fs: __fs, vol: __volume } = __memfs()
__fs.mkdirSync('/tmp', { recursive: true })
__fs.mkdirSync('/dev', { recursive: true })
__fs.writeFileSync('/dev/null', '')

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

const __wasmFile = await fetch(__wasmUrl).then((res) => res.arrayBuffer())

const {
	instance: __napiInstance,
	module: __wasiModule,
	napiModule: __napiModule,
} = __emnapiInstantiateNapiModuleSync(__wasmFile, {
	context: __emnapiContext,
	asyncWorkPoolSize: 4,
	wasi: __wasi,
	onCreateWorker() {
		const worker = new Worker(new URL('./wasi-worker-browser.js', import.meta.url), {
			type: 'module',
		})
		worker.addEventListener('message', __wasmCreateOnMessageForFsProxy(__fs))
		return worker
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
export default __napiModule.exports
export const Tagger = __napiModule.exports.Tagger
export const dictIndex = __napiModule.exports.dictIndex
export const mecabVersion = __napiModule.exports.mecabVersion
export const packDict = __napiModule.exports.packDict
