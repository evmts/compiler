import { describe, expect, it } from 'vitest'
import * as moduleExports from './index.js'
import { rspackPluginTevm as namedExport } from './index.js'

describe('index.js', () => {
	it('should export rspackPluginTevm correctly', () => {
		// Check named export exists
		expect(namedExport).toBeDefined()

		// Check the module exports structure
		expect(Object.keys(moduleExports)).toHaveLength(1)
		expect(moduleExports).toHaveProperty('rspackPluginTevm')
	})

	it('should include proper JSDoc documentation', async () => {
		// Get the file content directly to verify documentation
		const fs = await import('node:fs/promises')
		const path = await import('node:path')

		const filePath = path.resolve('./src/index.js')
		const fileContent = await fs.readFile(filePath, 'utf-8')

		// Check JSDoc documentation patterns
		expect(fileContent).toContain('@module')
		expect(fileContent).toContain('@example')

		// Check for Rspack-specific content
		expect(fileContent).toContain('rspack')
		expect(fileContent).toContain('Rust-based')
		expect(fileContent).toContain('rspackPluginTevm')
	})
})
