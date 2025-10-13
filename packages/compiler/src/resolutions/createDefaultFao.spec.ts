import { describe, expect, it } from 'vitest'
import { createDefaultFao } from './createDefaultFao.js'

describe('createDefaultFao', () => {
	const fao = createDefaultFao()

	describe('readFile', () => {
		it('should read an existing file', async () => {
			const content = await fao.readFile(__filename, 'utf-8')
			expect(content).toContain('createDefaultFao')
		})

		it('should throw for non-existent file', async () => {
			await expect(fao.readFile('/nonexistent/file.sol', 'utf-8')).rejects.toThrow()
		})
	})

	describe('readFileSync', () => {
		it('should read an existing file synchronously', () => {
			const content = fao.readFileSync(__filename, 'utf-8')
			expect(content).toContain('createDefaultFao')
		})

		it('should throw for non-existent file', () => {
			expect(() => fao.readFileSync('/nonexistent/file.sol', 'utf-8')).toThrow()
		})
	})

	describe('exists', () => {
		it('should return true for existing file', async () => {
			expect(await fao.exists(__filename)).toBe(true)
		})

		it('should return false for non-existent file', async () => {
			expect(await fao.exists('/nonexistent/file.sol')).toBe(false)
		})
	})

	describe('existsSync', () => {
		it('should return true for existing file', () => {
			expect(fao.existsSync(__filename)).toBe(true)
		})

		it('should return false for non-existent file', () => {
			expect(fao.existsSync('/nonexistent/file.sol')).toBe(false)
		})
	})
})
