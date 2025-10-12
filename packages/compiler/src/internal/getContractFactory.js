import { createContract } from '@tevm/contract'
import { CompilerOutputError } from './errors.js'

/**
 * Creates a getContract function for a compiled source
 *
 * @param {string} sourcePath - The source file path
 * @param {import('./CompiledSource.js').CompiledSource} output - The compiled source output
 * @param {import('@tevm/logger').Logger} logger - The logger
 * @returns {(contractName?: string) => import('@tevm/contract').Contract<string, readonly string[], undefined, import('@tevm/utils').Hex | undefined, import('@tevm/utils').Hex | undefined, undefined>}
 *
 * @example
 * ```javascript
 * import { getContractFactory } from './getContractFactory.js'
 *
 * const output = { contract: { MyContract: { abi: [...], evm: {...} } } }
 * const getContract = getContractFactory('Contract.sol', output, logger)
 *
 * // Get single contract
 * const contract = getContract()
 *
 * // Get specific contract from multiple
 * const contract = getContract('MyContract')
 * ```
 */
export const getContractFactory = (sourcePath, output, logger) => {
	/**
	 * Creates a Contract instance from the compiled source
	 * @param {string} [contractName] - Optional name of the contract
	 * @returns {import('@tevm/contract').Contract<string, readonly string[], undefined, import('@tevm/utils').Hex, import('@tevm/utils').Hex>}
	 * @throws {CompilerOutputError} If compilation output is missing required compilation data or contract not found
	 */
	return (contractName) => {
		const availableContractNames = Object.keys(output.contract)
		if (availableContractNames.length === 0) {
			const err = new CompilerOutputError(`No contracts found in source ${sourcePath}`, {
				meta: {
					code: 'no_contracts',
					sourcePath,
				},
			})
			logger.error(err.message)
			throw err
		}

		if (!contractName && availableContractNames.length > 1) {
			const err = new CompilerOutputError(
				`Multiple contracts found in source ${sourcePath}. Please specify a contract name. ` +
					`Available contracts: ${availableContractNames.join(', ')}`,
				{
					meta: {
						code: 'ambiguous_contract',
						sourcePath,
						availableContracts: availableContractNames,
					},
				},
			)
			logger.error(err.message)
			throw err
		}

		if (contractName && !availableContractNames.includes(contractName)) {
			const err = new CompilerOutputError(
				`Contract "${contractName}" not found in source ${sourcePath}. Available contracts: ${availableContractNames.join(', ')}`,
				{
					meta: {
						code: 'contract_not_found',
						contractName,
						sourcePath,
						availableContracts: availableContractNames,
					},
				},
			)
			logger.error(err.message)
			throw err
		}

		const selectedContractName = contractName ?? /** @type {string} */ (availableContractNames[0])
		if (!contractName) {
			logger.debug(`No contract name provided, using first available contract: ${selectedContractName}`)
		}

		if (!output.contract[selectedContractName]) {
			const err = new CompilerOutputError(
				`Contract "${selectedContractName}" not found in source ${sourcePath}. Available contracts: ${availableContractNames.join(', ')}`,
				{
					meta: {
						code: 'missing_compilation_output',
						contractName: selectedContractName,
						sourcePath,
						availableContracts: availableContractNames,
					},
				},
			)
			logger.error(err.message)
			throw err
		}

		const contractOutput = {
			abi: output.contract[selectedContractName].abi,
			bytecode: output.contract[selectedContractName].evm?.bytecode?.object,
			deployedBytecode: output.contract[selectedContractName].evm?.deployedBytecode?.object,
		}
		const missingFields = Object.entries(contractOutput)
			.filter(([_, value]) => !value)
			.map(([key]) => key)
		if (missingFields.length > 0) {
			const err = new CompilerOutputError(
				`Contract "${selectedContractName}" is missing ${missingFields.join(', ')} in compilation output`,
				{
					meta: {
						code: 'missing_compilation_output',
						contractName: selectedContractName,
						sourcePath,
						availableContracts: availableContractNames,
					},
				},
			)
			logger.error(err.message)
			throw err
		}

		return createContract({
			name: selectedContractName,
			abi: contractOutput.abi,
			bytecode: /** @type {import('@tevm/utils').Hex} */ (`0x${contractOutput.bytecode}`),
			deployedBytecode: /** @type {import('@tevm/utils').Hex} */ (`0x${contractOutput.deployedBytecode}`),
		})
	}
}
