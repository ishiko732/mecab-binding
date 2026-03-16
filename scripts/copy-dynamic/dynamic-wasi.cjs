/* eslint-disable */

const __nodeFs = require('node:fs')
const __nodePath = require('node:path')
const { WASI: __nodeWASI } = require('node:wasi')
const { Worker } = require('node:worker_threads')

const {
  createOnMessage: __wasmCreateOnMessageForFsProxy,
  getDefaultContext: __emnapiGetDefaultContext,
  instantiateNapiModuleSync: __emnapiInstantiateNapiModuleSync,
} = require('@napi-rs/wasm-runtime')

/**
 * Prevent a Node.js Worker from keeping the process alive.
 * @param {import('node:worker_threads').Worker} worker
 */
function _unrefWorker(worker) {
  const kPublicPort = Object.getOwnPropertySymbols(worker).find((s) =>
    s.toString().includes('kPublicPort'),
  )
  if (kPublicPort) {
    worker[kPublicPort].ref = () => {}
  }

  const kHandle = Object.getOwnPropertySymbols(worker).find((s) =>
    s.toString().includes('kHandle'),
  )
  if (kHandle) {
    worker[kHandle].ref = () => {}
  }

  worker.unref()
}

/**
 * Initialize the MeCab NAPI module with the given WASM binary and worker path.
 *
 * @param {object} options
 * @param {string | URL | BufferSource} options.wasm - Path to the .wasm file, URL, or raw binary.
 * @param {string | URL} [options.worker] - Path to the WASI worker script. If omitted, async work pool is disabled.
 * @returns {typeof import('./index')}
 */
function initMecab(options) {
  const { wasm, worker } = options

  const __rootDir = __nodePath.parse(process.cwd()).root

  const __wasi = new __nodeWASI({
    version: 'preview1',
    env: process.env,
    preopens: {
      [__rootDir]: __rootDir,
    },
  })

  const __emnapiContext = __emnapiGetDefaultContext()

  const __sharedMemory = new WebAssembly.Memory({
    initial: 4000,
    maximum: 65536,
    shared: true,
  })

  let wasmBinary
  if (typeof wasm === 'string') {
    wasmBinary = __nodeFs.readFileSync(wasm)
  } else if (wasm instanceof URL) {
    wasmBinary = __nodeFs.readFileSync(wasm)
  } else if (wasm instanceof ArrayBuffer || ArrayBuffer.isView(wasm)) {
    wasmBinary = wasm
  } else {
    throw new TypeError('options.wasm must be a file path (string), URL, or BufferSource')
  }

  const asyncWorkPoolSize = worker
    ? (function () {
        const threadsSizeFromEnv = Number(
          process.env.NAPI_RS_ASYNC_WORK_POOL_SIZE ?? process.env.UV_THREADPOOL_SIZE,
        )
        if (threadsSizeFromEnv > 0) {
          return threadsSizeFromEnv
        }
        return 4
      })()
    : 0

  const { napiModule: __napiModule } = __emnapiInstantiateNapiModuleSync(wasmBinary, {
    context: __emnapiContext,
    asyncWorkPoolSize,
    reuseWorker: true,
    wasi: __wasi,
    onCreateWorker() {
      if (!worker) {
        throw new Error('options.worker is required to use async work pool')
      }
      const workerPath = worker instanceof URL ? worker.pathname : String(worker)
      const w = new Worker(workerPath, {
        env: process.env,
      })
      w.onmessage = ({ data }) => {
        __wasmCreateOnMessageForFsProxy(__nodeFs)(data)
      }
      _unrefWorker(w)
      return w
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

module.exports = { initMecab }
