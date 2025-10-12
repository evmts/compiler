import { describe, expect, it } from 'vitest'
import { compilationOutputToSolcOutputSelection } from './compilationOutputToSolcOutputSelection.js'

describe('compilationOutputToSolcOutputSelection', () => {
	describe('passthrough options', () => {
		it('should pass through abi', () => {
			const result = compilationOutputToSolcOutputSelection(['abi'])
			expect(result).toEqual(['abi'])
		})

		it('should pass through ast', () => {
			const result = compilationOutputToSolcOutputSelection(['ast'])
			expect(result).toEqual(['ast'])
		})

		it('should pass through metadata', () => {
			const result = compilationOutputToSolcOutputSelection(['metadata'])
			expect(result).toEqual(['metadata'])
		})

		it('should pass through userdoc', () => {
			const result = compilationOutputToSolcOutputSelection(['userdoc'])
			expect(result).toEqual(['userdoc'])
		})

		it('should pass through devdoc', () => {
			const result = compilationOutputToSolcOutputSelection(['devdoc'])
			expect(result).toEqual(['devdoc'])
		})

		it('should pass through ir', () => {
			const result = compilationOutputToSolcOutputSelection(['ir'])
			expect(result).toEqual(['ir'])
		})

		it('should pass through irOptimized', () => {
			const result = compilationOutputToSolcOutputSelection(['irOptimized'])
			expect(result).toEqual(['irOptimized'])
		})

		it('should pass through storageLayout', () => {
			const result = compilationOutputToSolcOutputSelection(['storageLayout'])
			expect(result).toEqual(['storageLayout'])
		})

		it('should pass through wildcard', () => {
			const result = compilationOutputToSolcOutputSelection(['*'])
			expect(result).toEqual(['*'])
		})
	})

	describe('EVM options', () => {
		it('should convert bytecode to evm.bytecode', () => {
			const result = compilationOutputToSolcOutputSelection(['bytecode'])
			expect(result).toEqual(['evm.bytecode'])
		})

		it('should convert deployedBytecode to evm.deployedBytecode', () => {
			const result = compilationOutputToSolcOutputSelection(['deployedBytecode'])
			expect(result).toEqual(['evm.deployedBytecode'])
		})

		it('should convert assembly to evm.assembly', () => {
			const result = compilationOutputToSolcOutputSelection(['assembly'])
			expect(result).toEqual(['evm.assembly'])
		})

		it('should convert legacyAssembly to evm.legacyAssembly', () => {
			const result = compilationOutputToSolcOutputSelection(['legacyAssembly'])
			expect(result).toEqual(['evm.legacyAssembly'])
		})

		it('should convert gasEstimates to evm.gasEstimates', () => {
			const result = compilationOutputToSolcOutputSelection(['gasEstimates'])
			expect(result).toEqual(['evm.gasEstimates'])
		})

		it('should convert methodIdentifiers to evm.methodIdentifiers', () => {
			const result = compilationOutputToSolcOutputSelection(['methodIdentifiers'])
			expect(result).toEqual(['evm.methodIdentifiers'])
		})
	})

	describe('EWASM options', () => {
		it('should convert ewasm to ewasm.wasm and ewasm.wast', () => {
			const result = compilationOutputToSolcOutputSelection(['ewasm'])
			expect(result).toContain('ewasm.wasm')
			expect(result).toContain('ewasm.wast')
			expect(result).toHaveLength(2)
		})
	})

	describe('multiple options', () => {
		it('should handle multiple passthrough options', () => {
			const result = compilationOutputToSolcOutputSelection(['abi', 'metadata', 'userdoc'])
			expect(result).toContain('abi')
			expect(result).toContain('metadata')
			expect(result).toContain('userdoc')
			expect(result).toHaveLength(3)
		})

		it('should handle mixed passthrough and EVM options', () => {
			const result = compilationOutputToSolcOutputSelection(['abi', 'bytecode', 'metadata'])
			expect(result).toContain('abi')
			expect(result).toContain('evm.bytecode')
			expect(result).toContain('metadata')
			expect(result).toHaveLength(3)
		})

		it('should handle all EVM options together', () => {
			const result = compilationOutputToSolcOutputSelection([
				'bytecode',
				'deployedBytecode',
				'assembly',
				'gasEstimates',
				'methodIdentifiers',
			])
			expect(result).toContain('evm.bytecode')
			expect(result).toContain('evm.deployedBytecode')
			expect(result).toContain('evm.assembly')
			expect(result).toContain('evm.gasEstimates')
			expect(result).toContain('evm.methodIdentifiers')
			expect(result).toHaveLength(5)
		})

		it('should handle default compilation output', () => {
			const result = compilationOutputToSolcOutputSelection([
				'abi',
				'ast',
				'bytecode',
				'deployedBytecode',
				'storageLayout',
			])
			expect(result).toContain('abi')
			expect(result).toContain('ast')
			expect(result).toContain('evm.bytecode')
			expect(result).toContain('evm.deployedBytecode')
			expect(result).toContain('storageLayout')
			expect(result).toHaveLength(5)
		})
	})

	describe('edge cases', () => {
		it('should handle empty array', () => {
			const result = compilationOutputToSolcOutputSelection([])
			expect(result).toEqual([])
		})

		it('should deduplicate options', () => {
			const result = compilationOutputToSolcOutputSelection(['abi', 'abi', 'bytecode', 'bytecode'])
			expect(result).toContain('abi')
			expect(result).toContain('evm.bytecode')
			expect(result).toHaveLength(2)
		})

		it('should handle wildcard with other options (wildcard should be present)', () => {
			const result = compilationOutputToSolcOutputSelection(['*', 'abi', 'bytecode'])
			expect(result).toContain('*')
			// Other options may or may not be present, but wildcard should be
			expect(result.length).toBeGreaterThanOrEqual(1)
		})
	})

	describe('comprehensive example', () => {
		it('should convert complete output selection correctly', () => {
			const result = compilationOutputToSolcOutputSelection([
				'abi',
				'ast',
				'bytecode',
				'deployedBytecode',
				'metadata',
				'userdoc',
				'devdoc',
				'ir',
				'storageLayout',
				'assembly',
				'gasEstimates',
				'methodIdentifiers',
				'ewasm',
			])

			expect(result).toContain('abi')
			expect(result).toContain('ast')
			expect(result).toContain('evm.bytecode')
			expect(result).toContain('evm.deployedBytecode')
			expect(result).toContain('metadata')
			expect(result).toContain('userdoc')
			expect(result).toContain('devdoc')
			expect(result).toContain('ir')
			expect(result).toContain('storageLayout')
			expect(result).toContain('evm.assembly')
			expect(result).toContain('evm.gasEstimates')
			expect(result).toContain('evm.methodIdentifiers')
			expect(result).toContain('ewasm.wasm')
			expect(result).toContain('ewasm.wast')

			// Should have all unique options (ewasm expands to 2)
			expect(result).toHaveLength(14)
		})
	})
})
