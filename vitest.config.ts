import { resolve } from 'node:path'
import { defineConfig } from 'vitest/config'

export default defineConfig({
	assetsInclude: ['**/*.data'],
	resolve: {
		alias: [
			{ find: 'mecab-binding/ipadic.data', replacement: resolve(__dirname, 'dist', 'ipadic.data') },
			{ find: /^mecab-binding$/, replacement: resolve(__dirname, 'dist', 'index.js') },
		],
	},
	test: {
		testTimeout: 120_000,
	},
})
