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
