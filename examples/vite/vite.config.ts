import path from 'node:path'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

const wasiExports = {
	'mecab-binding-wasm32-wasi/wasm': path.resolve(__dirname, '../../dist/mecab-binding.wasm32-wasi.wasm'),
	'mecab-binding-wasm32-wasi/wasi-worker': path.resolve(__dirname, '../../dist/wasi-worker-browser.mjs'),
}

export default defineConfig({
	plugins: [
		react(),
		{
			name: 'mecab-wasi-resolver',
			resolveId(id) {
				const [bare, query] = id.split('?')
				if (bare in wasiExports) {
					const resolved = wasiExports[bare as keyof typeof wasiExports]
					return query ? `${resolved}?${query}` : resolved
				}
			},
		},
	],
	assetsInclude: ['**/*.data'],
	resolve: {
		alias: [
			{ find: /^mecab-binding$/, replacement: path.resolve(__dirname, '../../dist/mecab-binding.wasi-browser.js') },
		],
	},
	build: {
		target: 'esnext',
	},
	server: {
		headers: {
			'Cross-Origin-Opener-Policy': 'same-origin',
			'Cross-Origin-Embedder-Policy': 'require-corp',
		},
		fs: {
			allow: [path.resolve(__dirname, '../..')],
		},
	},
	optimizeDeps: {
		exclude: ['mecab-binding'],
	},
})
