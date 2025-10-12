import type { SolcErrorEntry, SolcInputDescription } from '@tevm/solc'

export interface CompileBaseResult {
	errors?: SolcErrorEntry[] | undefined
	solcInput: SolcInputDescription
}
