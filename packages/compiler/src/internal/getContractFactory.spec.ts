import type { Logger } from '@tevm/logger'
import { describe, expect, it, vi } from 'vitest'
import type { CompiledSource } from './CompiledSource.js'
import { CompilerOutputError } from './errors.js'
import { getContractFactory } from './getContractFactory.js'

describe('getContractFactory', () => {
	const mockAbi = [{ type: 'function', name: 'test', inputs: [], outputs: [], stateMutability: 'pure' }] as const

	const mockOutput = {
		contract: {
			TestContract: {
				abi: mockAbi,
				evm: {
					bytecode: { object: '6080604052' },
					deployedBytecode: { object: '60806040' },
				},
			},
		},
	} as unknown as CompiledSource

	const mockLogger = {
		error: vi.fn(),
		debug: vi.fn(),
	} as unknown as Logger

	describe('happy path', () => {
		it('should create getContract function', () => {
			const getContract = getContractFactory('Contract.sol', mockOutput, mockLogger)
			expect(getContract).toBeTypeOf('function')
		})

		it('should return contract for single contract without name', () => {
			const getContract = getContractFactory('Contract.sol', mockOutput, mockLogger)
			const contract = getContract()

			expect(contract.name).toBe('TestContract')
			expect(contract.abi).toEqual(mockAbi)
			expect(contract.bytecode).toBe('0x6080604052')
			expect(contract.deployedBytecode).toBe('0x60806040')
		})

		it('should return contract with specified name', () => {
			const getContract = getContractFactory('Contract.sol', mockOutput, mockLogger)
			const contract = getContract('TestContract')

			expect(contract.name).toBe('TestContract')
		})

		it('should return correct contract for multiple contracts', () => {
			const multiOutput = {
				contract: {
					ContractA: { abi: mockAbi, evm: { bytecode: { object: 'aa' }, deployedBytecode: { object: 'bb' } } },
					ContractB: { abi: mockAbi, evm: { bytecode: { object: 'cc' }, deployedBytecode: { object: 'dd' } } },
				},
			} as unknown as CompiledSource

			const getContract = getContractFactory('Multi.sol', multiOutput, mockLogger)
			const contractA = getContract('ContractA')
			const contractB = getContract('ContractB')

			expect(contractA.name).toBe('ContractA')
			expect(contractA.bytecode).toBe('0xaa')
			expect(contractB.name).toBe('ContractB')
			expect(contractB.bytecode).toBe('0xcc')
		})
	})

	describe('error handling', () => {
		it('should throw when no contracts exist', () => {
			const emptyOutput = { contract: {} } as unknown as CompiledSource
			const getContract = getContractFactory('Empty.sol', emptyOutput, mockLogger)

			expect(() => getContract()).toThrow(CompilerOutputError)
			expect(() => getContract()).toThrow(/No contracts found/)

			try {
				getContract()
			} catch (error) {
				if (error instanceof CompilerOutputError) {
					expect(error.meta?.code).toBe('no_contracts')
					expect(error.meta?.sourcePath).toBe('Empty.sol')
				}
			}
		})

		it('should throw when name not specified for multiple contracts', () => {
			const multiOutput = {
				contract: {
					A: { abi: mockAbi, evm: { bytecode: { object: 'aa' }, deployedBytecode: { object: 'bb' } } },
					B: { abi: mockAbi, evm: { bytecode: { object: 'cc' }, deployedBytecode: { object: 'dd' } } },
				},
			} as unknown as CompiledSource

			const getContract = getContractFactory('Multi.sol', multiOutput, mockLogger)

			expect(() => getContract()).toThrow(CompilerOutputError)
			expect(() => getContract()).toThrow(/Multiple contracts found/)

			try {
				getContract()
			} catch (error) {
				if (error instanceof CompilerOutputError) {
					expect(error.meta?.code).toBe('ambiguous_contract')
					expect(error.meta?.availableContracts).toEqual(['A', 'B'])
				}
			}
		})

		it('should throw when contract name not found', () => {
			const getContract = getContractFactory('Contract.sol', mockOutput, mockLogger)

			expect(() => getContract('NonExistent')).toThrow(CompilerOutputError)
			expect(() => getContract('NonExistent')).toThrow(/not found/)

			try {
				getContract('NonExistent')
			} catch (error) {
				if (error instanceof CompilerOutputError) {
					expect(error.meta?.code).toBe('contract_not_found')
					expect(error.meta?.contractName).toBe('NonExistent')
					expect(error.meta?.availableContracts).toEqual(['TestContract'])
				}
			}
		})

		it('should throw when ABI is missing', () => {
			const noAbiOutput = {
				contract: {
					TestContract: {
						evm: { bytecode: { object: 'aa' }, deployedBytecode: { object: 'bb' } },
					},
				},
			} as unknown as CompiledSource

			const getContract = getContractFactory('Contract.sol', noAbiOutput, mockLogger)

			expect(() => getContract()).toThrow(CompilerOutputError)
			expect(() => getContract()).toThrow(/missing abi/)

			try {
				getContract()
			} catch (error) {
				if (error instanceof CompilerOutputError) {
					expect(error.meta?.code).toBe('missing_compilation_output')
				}
			}
		})

		it('should throw when bytecode is missing', () => {
			const noBytecodeOutput = {
				contract: {
					TestContract: {
						abi: mockAbi,
						evm: { deployedBytecode: { object: 'bb' } },
					},
				},
			} as unknown as CompiledSource

			const getContract = getContractFactory('Contract.sol', noBytecodeOutput, mockLogger)

			expect(() => getContract()).toThrow(CompilerOutputError)
			expect(() => getContract()).toThrow(/missing bytecode/)

			try {
				getContract()
			} catch (error) {
				if (error instanceof CompilerOutputError) {
					expect(error.meta?.code).toBe('missing_compilation_output')
				}
			}
		})

		it('should throw when deployed bytecode is missing', () => {
			const noDeployedBytecodeOutput = {
				contract: {
					TestContract: {
						abi: mockAbi,
						evm: { bytecode: { object: 'aa' } },
					},
				},
			} as unknown as CompiledSource

			const getContract = getContractFactory('Contract.sol', noDeployedBytecodeOutput, mockLogger)

			expect(() => getContract()).toThrow(CompilerOutputError)
			expect(() => getContract()).toThrow(/missing deployedBytecode/)

			try {
				getContract()
			} catch (error) {
				if (error instanceof CompilerOutputError) {
					expect(error.meta?.code).toBe('missing_compilation_output')
				}
			}
		})
	})
})
