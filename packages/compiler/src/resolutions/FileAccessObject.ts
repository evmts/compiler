/**
 * File Access Object (FAO) interface for abstracting filesystem operations.
 *
 * Allows the compiler to work with virtual filesystems, custom sources,
 * or mocked implementations for testing.
 *
 * @example
 * ```typescript
 * import { readFile, readFileSync, existsSync } from 'node:fs'
 * import { access } from 'node:fs/promises'
 *
 * const defaultFao: FileAccessObject = {
 *   readFile: (path, encoding) => readFile(path, { encoding }),
 *   readFileSync: (path, encoding) => readFileSync(path, { encoding }),
 *   existsSync: (path) => existsSync(path),
 *   exists: async (path) => {
 *     try {
 *       await access(path)
 *       return true
 *     } catch {
 *       return false
 *     }
 *   }
 * }
 * ```
 */
export interface FileAccessObject {
	/**
	 * Read a file asynchronously
	 * @param path - Absolute or relative path to the file
	 * @param encoding - Character encoding (typically 'utf-8')
	 * @returns Promise resolving to file contents
	 */
	readFile: (path: string, encoding: BufferEncoding) => Promise<string>

	/**
	 * Read a file synchronously
	 * @param path - Absolute or relative path to the file
	 * @param encoding - Character encoding (typically 'utf-8')
	 * @returns File contents
	 */
	readFileSync: (path: string, encoding: BufferEncoding) => string

	/**
	 * Check if a file exists synchronously
	 * @param path - Absolute or relative path to check
	 * @returns true if file exists, false otherwise
	 */
	existsSync: (path: string) => boolean

	/**
	 * Check if a file exists asynchronously
	 * @param path - Absolute or relative path to check
	 * @returns Promise resolving to true if file exists, false otherwise
	 */
	exists: (path: string) => Promise<boolean>
}
