import type * as binding from './index'

export interface InitMecabOptions {
  /**
   * The WASM binary to instantiate.
   *
   * - **Node.js**: a file path (`string`), `URL`, or `BufferSource`.
   * - **Browser**: a `BufferSource`, `WebAssembly.Module`, `URL`, or `string` (fetched).
   */
  wasm: string | URL | BufferSource | WebAssembly.Module

  /**
   * Path or URL to the WASI worker script.
   *
   * When omitted the async work pool is disabled (pool size = 0).
   *
   * - **Node.js**: file path (`string`) or `URL`.
   * - **Browser**: `URL` or `string` resolved against `import.meta.url`.
   */
  worker?: string | URL
}

/**
 * Dynamically initialize the MeCab binding with user-supplied WASM and worker resources.
 *
 * @example
 * ```ts
 * // Node.js (CommonJS)
 * const { initMecab } = require('mecab-binding/dynamic-wasi')
 * const binding = initMecab({
 *   wasm: require.resolve('mecab-binding/mecab.wasm'),
 *   worker: require.resolve('mecab-binding/dist/wasi-worker.mjs'),
 * })
 * ```
 *
 * @example
 * ```ts
 * // Browser (ES module)
 * import { initMecab } from 'mecab-binding/dynamic-wasi'
 * const binding = await initMecab({
 *   wasm: new URL('mecab-binding/mecab.wasm', import.meta.url),
 *   worker: new URL('mecab-binding/dist/wasi-worker-browser.mjs', import.meta.url),
 * })
 * ```
 */
export declare function initMecab(options: InitMecabOptions): Promise<typeof binding>
