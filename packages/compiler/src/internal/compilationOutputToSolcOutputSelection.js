/**
 * Convert flat compilation output options to Solc's nested format
 *
 * Maps our user-friendly flat API to Solc's nested structure:
 * - 'bytecode' → 'evm.bytecode'
 * - 'deployedBytecode' → 'evm.deployedBytecode'
 * - 'assembly' → 'evm.assembly'
 * - 'legacyAssembly' → 'evm.legacyAssembly'
 * - 'gasEstimates' → 'evm.gasEstimates'
 * - 'methodIdentifiers' → 'evm.methodIdentifiers'
 * - 'ewasm' → ['ewasm.wasm', 'ewasm.wast']
 * - Others pass through: 'abi', 'ast', 'metadata', 'userdoc', 'devdoc', 'ir', 'storageLayout', '*'
 *
 * @param {readonly import('../compile/CompilationOutputOption.js').CompilationOutputOption[]} compilationOutput - Flat compilation output options
 * @returns {Array<import('@tevm/solc').SolcOutputSelection['*']['*'][number]>} Solc-compatible nested output selection
 *
 * @example
 * compilationOutputToSolcOutputSelection(['abi', 'bytecode'])
 * // Returns: ['abi', 'evm.bytecode']
 *
 * @example
 * compilationOutputToSolcOutputSelection(['ewasm'])
 * // Returns: ['ewasm.wasm', 'ewasm.wast']
 */
export const compilationOutputToSolcOutputSelection = (compilationOutput) => {
	/** @type {Set<import('@tevm/solc').SolcOutputSelection['*']['*'][number]>} */
	const solcOutputSelection = new Set()

	for (const option of compilationOutput) {
		switch (option) {
			// EVM options - prefix with 'evm.'
			case 'bytecode':
				solcOutputSelection.add('evm.bytecode')
				break
			case 'deployedBytecode':
				solcOutputSelection.add('evm.deployedBytecode')
				break
			case 'assembly':
				solcOutputSelection.add('evm.assembly')
				break
			case 'legacyAssembly':
				solcOutputSelection.add('evm.legacyAssembly')
				break
			case 'gasEstimates':
				solcOutputSelection.add('evm.gasEstimates')
				break
			case 'methodIdentifiers':
				solcOutputSelection.add('evm.methodIdentifiers')
				break
			// EWASM - expands to both wasm and wast
			case 'ewasm':
				solcOutputSelection.add('ewasm.wasm')
				solcOutputSelection.add('ewasm.wast')
				break
			// All other options pass through as-is: *, abi, ast, metadata, userdoc, devdoc, ir, irOptimized, storageLayout
			default:
				solcOutputSelection.add(option)
				break
		}
	}

	return Array.from(solcOutputSelection)
}
