import type { SolcContractOutput } from '@tevm/solc'
import type { Abi } from 'abitype'
import type { CompilationOutputOption } from '../compile/CompilationOutputOption.js'

export type CompiledSourceContractOutput<
	TOutputSelection extends readonly CompilationOutputOption[] | undefined =
		| readonly CompilationOutputOption[]
		| undefined,
> =
	// Wrap in [] to prevent distribution over union types
	// Check if it's a specific array type (not the union)
	[TOutputSelection] extends [readonly CompilationOutputOption[]]
		? // It's a specific array - check for '*'
			Extract<TOutputSelection[number], '*'> extends never
			? // No '*', build output from selected fields
				WithCompilationOutput<TOutputSelection, 'abi', { abi: Abi }> &
					WithCompilationOutput<TOutputSelection, 'metadata', { metadata: string }> &
					WithCompilationOutput<TOutputSelection, 'userdoc', { userdoc: SolcContractOutput['userdoc'] }> &
					WithCompilationOutput<TOutputSelection, 'devdoc', { devdoc: SolcContractOutput['devdoc'] }> &
					WithCompilationOutput<TOutputSelection, 'ir', { ir: string }> &
					WithCompilationOutput<
						TOutputSelection,
						'storageLayout',
						{ storageLayout: SolcContractOutput['storageLayout'] }
					> &
					WithCompilationOutput<TOutputSelection, 'ewasm', { ewasm: SolcContractOutput['ewasm'] }> &
					// EVM options - conditionally include evm object if any EVM-related option is selected
					WithEvmOutput<TOutputSelection>
			: // Has '*', return everything
				SolcContractOutput
		: // It's undefined or the full union - return defaults
			{ abi: Abi; evm: SolcContractOutput['evm']; storageLayout: SolcContractOutput['storageLayout'] }

/**
 * Helper type to conditionally include the evm object if any EVM-related options are selected
 * Maps flat options (bytecode, assembly, etc.) to the nested evm structure
 * @template TOutputSelection - The compilation output selection array
 */
type WithEvmOutput<TOutputSelection extends readonly CompilationOutputOption[]> =
	// Check if any EVM-related option is selected
	Extract<
		TOutputSelection[number],
		'bytecode' | 'deployedBytecode' | 'assembly' | 'legacyAssembly' | 'gasEstimates' | 'methodIdentifiers'
	> extends never
		? {} // No EVM options selected
		: {
				evm: WithCompilationOutput<TOutputSelection, 'bytecode', { bytecode: SolcContractOutput['evm']['bytecode'] }> &
					WithCompilationOutput<
						TOutputSelection,
						'deployedBytecode',
						{ deployedBytecode: SolcContractOutput['evm']['deployedBytecode'] }
					> &
					WithCompilationOutput<TOutputSelection, 'assembly', { assembly: SolcContractOutput['evm']['assembly'] }> &
					WithCompilationOutput<
						TOutputSelection,
						'legacyAssembly',
						{ legacyAssembly: SolcContractOutput['evm']['legacyAssembly'] }
					> &
					WithCompilationOutput<
						TOutputSelection,
						'gasEstimates',
						{ gasEstimates: SolcContractOutput['evm']['gasEstimates'] }
					> &
					WithCompilationOutput<
						TOutputSelection,
						'methodIdentifiers',
						{ methodIdentifiers: SolcContractOutput['evm']['methodIdentifiers'] }
					>
			}

/**
 * Helper type to conditionally include an object in the output based on compilation output selection
 * Uses Extract to check if ANY member of the selection union matches the path
 * @template TOutputSelection - The compilation output selection array
 * @template Path - The path to check in the selection (e.g., 'abi', 'bytecode', 'metadata')
 * @template Output - The object to include if the path is selected
 */
type WithCompilationOutput<
	TOutputSelection extends readonly string[],
	Path extends string,
	Output extends object,
> = Extract<TOutputSelection[number], Path> extends never ? {} : Output
