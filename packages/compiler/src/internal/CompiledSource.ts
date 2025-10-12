import type { SolcAst } from '@tevm/solc'
import type { CompilationOutputOption } from '../compile/CompilationOutputOption.js'
import type { CompiledSourceContractOutput } from './CompiledSourceContractOutput.js'

export interface CompiledSource<
	TCompilationOutput extends CompilationOutputOption[] | undefined = CompilationOutputOption[] | undefined,
> {
	ast: Extract<
		TCompilationOutput extends CompilationOutputOption[] ? TCompilationOutput[number] : never,
		'ast' | '*'
	> extends never
		? undefined
		: SolcAst
	id: number
	contract: {
		[sourceName: string]: CompiledSourceContractOutput<TCompilationOutput>
	}
	/**
	 * Gets a Contract instance from the compiled source.
	 *
	 * @param contractName - Optional name of the contract. If not provided and there's only one contract, that contract will be used.
	 *                       If not provided and there are multiple contracts, an error will be thrown.
	 * @returns A Contract instance from @tevm/contract
	 * @throws {CompilerOutputError} If required compilation outputs (abi, bytecode, deployedBytecode) are missing
	 * @throws {CompilerOutputError} If contractName is not provided and there are multiple contracts
	 * @throws {CompilerOutputError} If contractName is provided but not found in the source
	 * @throws {CompilerOutputError} If no contracts exist in the source
	 *
	 * @example
	 * ```typescript
	 * import { Compiler } from '@tevm/compiler'
	 *
	 * // ...
	 * const { compilationResult } = await Compiler.compileSource(`contract Token { ... }`)
	 * const Token = compilationResult.getContract()
	 * console.log(Token.read.balanceOf('0x...'))
	 * // -> { abi: [...], functionName: 'balanceOf', args: ['0x...'] }
	 * ```
	 */
	getContract: (
		contractName?: string,
	) => import('@tevm/contract').Contract<
		string,
		readonly string[],
		undefined,
		import('@tevm/utils').Hex,
		import('@tevm/utils').Hex
	>
}
