import path from 'node:path'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
	plugins: [react()],
	assetsInclude: ['**/*.data'],
	resolve: {
		alias: [{ find: /^mecab-binding$/, replacement: path.resolve(__dirname, 'src/mecab-binding.js') }],
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
