import type { Logger } from '@tevm/logger'
import type { Solc } from '@tevm/solc'
import type { CompileBaseOptions } from './compile/CompileBaseOptions.js'

export interface CreateCompilerOptions extends Omit<CompileBaseOptions, 'solcVersion'> {
	solc?: Solc
	/**
	 * Passing a custom logger will override any `loggingLevel` setting
	 */
	logger?: Logger
}
