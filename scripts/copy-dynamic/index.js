import { copyFileSync, mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const rootDir = join(__dirname, '..', '..')
const srcDir = __dirname
const destDir = join(rootDir, 'dist')

mkdirSync(destDir, { recursive: true })

const files = ['dynamic-wasi-browser.js', 'dynamic-wasi.cjs', 'dynamic-wasi.d.ts']

for (const file of files) {
  copyFileSync(join(srcDir, file), join(destDir, file))
  console.log(`Copied ${file} -> dist/${file}`)
}

// Replace NAPI-RS auto-generated browser WASI files with memfs-enabled versions
const overrides = {
  'mecab-binding.wasi-browser.js': 'mecab-binding.wasi-browser.js',
  'wasi-worker-browser.mjs': 'wasi-worker-browser.mjs',
}

for (const [src, dest] of Object.entries(overrides)) {
  copyFileSync(join(srcDir, src), join(destDir, dest))
  console.log(`Replaced dist/${dest}`)
}
