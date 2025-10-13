import { existsSync, readFileSync } from 'node:fs'
import { access, readFile } from 'node:fs/promises'

/**
 * Create a default FileAccessObject using node:fs module
 *
 * This is the default implementation used when no custom FAO is provided.
 * It directly wraps Node.js filesystem operations.
 *
 * @returns {import('./FileAccessObject.js').FileAccessObject}
 * @example
 * import { createDefaultFao } from './resolutions/createDefaultFao.js'
 *
 * const fao = createDefaultFao()
 * const content = await fao.readFile('./contract.sol', 'utf-8')
 * console.log(content)
 */
export const createDefaultFao = () => {
	return {
		readFile: (path, encoding) => readFile(path, encoding),
		readFileSync: (path, encoding) => readFileSync(path, encoding),
		exists: async (path) => {
			try {
				await access(path)
				return true
			} catch {
				return false
			}
		},
		existsSync: (path) => existsSync(path),
	}
}
