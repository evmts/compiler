import type { Logger } from '@tevm/logger'
import { assert, beforeEach, describe, expect, it, vi } from 'vitest'
import { SimpleContract, SimpleYul } from '../../fixtures/index.js'
import { compileContracts } from './compileContracts.js'
import { CompilerOutputError } from './errors.js'
import type { ValidatedCompileBaseOptions } from './ValidatedCompileBaseOptions.js'

describe('compileContracts', () => {
	const solc = require('solc')
	const mockLogger = {
		debug: vi.fn(),
		info: vi.fn(),
		warn: vi.fn(),
		error: vi.fn(),
	} as unknown as Logger

	beforeEach(() => {
		vi.clearAllMocks()
	})

	describe('single contract compilation', () => {
		it('should compile a single contract successfully', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['ast', 'abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.solcInput).toBeDefined()
			expect(result.solcInput.language).toBe('Solidity')
			expect(result.solcInput.sources['Contract.sol']).toBeDefined()
			expect(result.solcInput.settings).toBeDefined()
			assert(result.compilationResult['Contract.sol'], 'Contract.sol not found')
			expect(result.compilationResult['Contract.sol'].ast).toBeDefined()
			expect(result.compilationResult['Contract.sol'].id).toBeDefined()
			expect(result.compilationResult['Contract.sol'].contract['TestContract']).toBeDefined()
			expect(mockLogger.debug).toHaveBeenCalled()
		})

		it('should compile without AST when ast is not in compilation output', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			assert(result.compilationResult['Contract.sol'], 'Contract.sol not found')
			expect(result.compilationResult['Contract.sol'].ast).toBeUndefined()
			expect(result.compilationResult['Contract.sol'].contract['TestContract']).toBeDefined()
		})
	})

	describe('multiple contracts compilation', () => {
		it('should compile multiple contracts successfully', () => {
			const sources = {
				'Contract1.sol': 'pragma solidity ^0.8.0; contract Contract1 {}',
				'Contract2.sol': 'pragma solidity ^0.8.0; contract Contract2 {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['ast', 'abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.solcInput).toBeDefined()
			expect(result.solcInput.language).toBe('Solidity')
			expect(result.solcInput.sources['Contract1.sol']).toBeDefined()
			expect(result.solcInput.sources['Contract2.sol']).toBeDefined()
			expect(Object.keys(result.compilationResult)).toHaveLength(2)
			expect(result.compilationResult['Contract1.sol']).toBeDefined()
			expect(result.compilationResult['Contract2.sol']).toBeDefined()
		})

		it('should compile contract with multiple nested contracts', () => {
			const sources = {
				'Multi.sol': 'pragma solidity ^0.8.0; contract A {} contract B {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['ast', 'abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Multi.sol']?.contract['A']).toBeDefined()
			expect(result.compilationResult['Multi.sol']?.contract['B']).toBeDefined()
		})
	})

	describe('compilation errors', () => {
		it('should handle compilation errors and not throw when throwOnCompilationError is false', () => {
			const sources = {
				'Invalid.sol': 'pragma solidity ^0.8.0; contract Invalid { invalid }',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['ast', 'abi'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.solcInput).toBeDefined()
			expect(result.errors).toBeDefined()
			expect(result.errors!.length).toBeGreaterThan(0)
			expect(mockLogger.error).toHaveBeenCalled()
		})

		it('should throw CompilerOutputError when throwOnCompilationError is true', () => {
			const sources = {
				'Invalid.sol': 'pragma solidity ^0.8.0; contract Invalid { invalid }',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['ast', 'abi'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: true,
			}

			expect(() => compileContracts(solc, sources, options, mockLogger)).toThrow(CompilerOutputError)
		})

		it('should handle multiple compilation errors', () => {
			const sources = {
				'Invalid.sol': 'pragma solidity ^0.8.0; contract Invalid { invalid; other }',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['ast', 'abi'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.errors).toBeDefined()
			expect(result.errors!.length).toBeGreaterThan(0)
		})
	})

	describe('different output selections', () => {
		it('should compile with only abi output', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['abi'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']?.contract['TestContract']).toBeDefined()
			expect(result.compilationResult['Contract.sol']?.ast).toBeUndefined()
		})

		it('should compile with bytecode and deployedBytecode', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options = {
				language: 'Solidity',
				compilationOutput: ['bytecode', 'deployedBytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			} as const satisfies ValidatedCompileBaseOptions

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']?.contract['TestContract']?.evm?.bytecode).toBeDefined()
			expect(result.compilationResult['Contract.sol']?.contract['TestContract']?.evm?.deployedBytecode).toBeDefined()
		})

		it('should compile with all output selections using *', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['*'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']).toBeDefined()
		})
	})

	describe('optimizer settings', () => {
		it('should compile with optimizer enabled', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
				optimizer: {
					enabled: true,
					runs: 200,
				},
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']).toBeDefined()
		})

		it('should compile with detailed optimizer settings', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
				optimizer: {
					enabled: true,
					runs: 1000,
					details: {
						yul: true,
						yulDetails: {
							stackAllocation: true,
						},
					},
				},
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']).toBeDefined()
		})
	})

	describe('viaIR', () => {
		it('should compile with viaIR enabled', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
				viaIR: true,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']).toBeDefined()
		})

		it('should compile with viaIR disabled', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
				viaIR: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']).toBeDefined()
		})
	})

	describe('remappings', () => {
		it('should compile with remappings', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
				remappings: ['@openzeppelin/=node_modules/@openzeppelin/', '@libs/=contracts/libraries/'],
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']).toBeDefined()
		})
	})

	describe('libraries', () => {
		it('should compile with library addresses', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
				libraries: {
					'Library.sol': {
						Library: '0x1234567890123456789012345678901234567890',
					},
				},
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']).toBeDefined()
		})

		it('should compile with multiple libraries', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
				libraries: {
					'Lib1.sol': {
						Lib1: '0x1111111111111111111111111111111111111111',
					},
					'Lib2.sol': {
						Lib2: '0x2222222222222222222222222222222222222222',
					},
				},
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']).toBeDefined()
		})
	})

	describe('additional settings', () => {
		it('should compile with debug settings', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
				debug: {
					revertStrings: 'debug',
				},
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']).toBeDefined()
		})

		it('should compile with metadata settings', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
				metadata: {
					useLiteralContent: true,
				},
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']).toBeDefined()
		})

		it('should compile with model checker settings', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
				modelChecker: {
					engine: 'all',
				},
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']).toBeDefined()
		})
	})

	describe('language support', () => {
		it('should compile Yul code', () => {
			const sources = {
				'Contract.yul': '{ sstore(0, 1) }',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Yul',
				compilationOutput: ['bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.yul']).toBeDefined()
		})

		it('should compile Yul code with AST output', () => {
			const sources = {
				'SimpleYul.yul': SimpleYul.source,
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Yul',
				compilationOutput: ['ast', 'bytecode', 'deployedBytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.solcInput).toBeDefined()
			expect(result.solcInput.language).toBe('Yul')
			expect(result.solcInput.sources['SimpleYul.yul']).toBeDefined()
			assert(result.compilationResult['SimpleYul.yul'], 'SimpleYul.yul not found')
			expect(result.compilationResult['SimpleYul.yul'].ast).toBeDefined()
			expect(result.compilationResult['SimpleYul.yul'].ast?.nodeType).toBe('YulObject')
			expect(result.compilationResult['SimpleYul.yul'].id).toBeDefined()
			expect(result.compilationResult['SimpleYul.yul'].contract['SimpleYul']).toBeDefined()
			expect(result.compilationResult['SimpleYul.yul'].contract['SimpleYul']?.evm?.bytecode).toBeDefined()
			expect(result.compilationResult['SimpleYul.yul'].contract['SimpleYul']?.evm?.deployedBytecode).toBeDefined()
		})

		it('should compile from Solidity AST', () => {
			// Get the AST from SimpleContract fixture
			const ast = SimpleContract.solcOutput.sources!['SimpleContract.sol']!.ast

			const sources = {
				'SimpleContract.sol': ast,
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'SolidityAST',
				compilationOutput: ['abi', 'bytecode'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.solcInput).toBeDefined()
			expect(result.solcInput.language).toBe('SolidityAST')
			expect(result.solcInput.sources['SimpleContract.sol']).toBeDefined()
			assert(result.compilationResult['SimpleContract.sol'], 'SimpleContract.sol not found')
			expect(result.compilationResult['SimpleContract.sol'].contract['SimpleContract']).toBeDefined()
			expect(result.compilationResult['SimpleContract.sol'].contract['SimpleContract']?.abi).toBeDefined()
			expect(result.compilationResult['SimpleContract.sol'].contract['SimpleContract']?.evm?.bytecode).toBeDefined()
		})
	})

	describe('edge cases', () => {
		it('should handle empty contracts output gracefully', () => {
			const sources = {
				'Empty.sol': 'pragma solidity ^0.8.0;',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['ast', 'abi'],
				hardfork: 'cancun',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Empty.sol']).toBeDefined()
			expect(result.compilationResult['Empty.sol']?.ast).toBeDefined()
		})

		it('should handle different hardfork versions', () => {
			const sources = {
				'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
			}

			const options: ValidatedCompileBaseOptions = {
				language: 'Solidity',
				compilationOutput: ['abi', 'bytecode'],
				hardfork: 'shanghai',
				solcVersion: '0.8.28',
				throwOnVersionMismatch: false,
				throwOnCompilationError: false,
			}

			const result = compileContracts(solc, sources, options, mockLogger)

			expect(result.compilationResult['Contract.sol']).toBeDefined()
		})
	})

	describe('getContract', () => {
		describe('happy path', () => {
			it('should create contract instance for single contract without name', () => {
				const sources = {
					'Contract.sol':
						'pragma solidity ^0.8.0; contract TestContract { function test() public pure returns (uint256) { return 42; } }',
				}

				const options: ValidatedCompileBaseOptions = {
					language: 'Solidity',
					compilationOutput: ['abi', 'bytecode', 'deployedBytecode'],
					hardfork: 'cancun',
					solcVersion: '0.8.28',
					throwOnVersionMismatch: false,
					throwOnCompilationError: false,
				}

				const result = compileContracts(solc, sources, options, mockLogger)
				const source = result.compilationResult['Contract.sol']
				assert(source, 'Source not found')

				const contract = source.getContract()

				expect(contract).toBeDefined()
				expect(contract.name).toBe('TestContract')
				expect(contract.abi).toBeDefined()
				expect(contract.bytecode).toBeDefined()
				expect(contract.bytecode).toMatch(/^0x[0-9a-fA-F]+$/)
				expect(contract.deployedBytecode).toBeDefined()
				expect(contract.deployedBytecode).toMatch(/^0x[0-9a-fA-F]+$/)
				expect(contract.read).toBeDefined()
				expect(contract.write).toBeDefined()
				expect(contract.events).toBeDefined()
			})

			it('should create contract instance with name for single contract', () => {
				const sources = {
					'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
				}

				const options: ValidatedCompileBaseOptions = {
					language: 'Solidity',
					compilationOutput: ['abi', 'bytecode', 'deployedBytecode'],
					hardfork: 'cancun',
					solcVersion: '0.8.28',
					throwOnVersionMismatch: false,
					throwOnCompilationError: false,
				}

				const result = compileContracts(solc, sources, options, mockLogger)
				const source = result.compilationResult['Contract.sol']
				assert(source, 'Source not found')

				const contract = source.getContract('TestContract')

				expect(contract).toBeDefined()
				expect(contract.name).toBe('TestContract')
			})

			it('should create contract instance for multiple contracts with name specified', () => {
				const sources = {
					'Multi.sol': 'pragma solidity ^0.8.0; contract A {} contract B {}',
				}

				const options: ValidatedCompileBaseOptions = {
					language: 'Solidity',
					compilationOutput: ['abi', 'bytecode', 'deployedBytecode'],
					hardfork: 'cancun',
					solcVersion: '0.8.28',
					throwOnVersionMismatch: false,
					throwOnCompilationError: false,
				}

				const result = compileContracts(solc, sources, options, mockLogger)
				const source = result.compilationResult['Multi.sol']
				assert(source, 'Source not found')

				const contractA = source.getContract('A')
				expect(contractA.name).toBe('A')

				const contractB = source.getContract('B')
				expect(contractB.name).toBe('B')
			})

			it('should work with * compilation output', () => {
				const sources = {
					'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
				}

				const options: ValidatedCompileBaseOptions = {
					language: 'Solidity',
					compilationOutput: ['*'],
					hardfork: 'cancun',
					solcVersion: '0.8.28',
					throwOnVersionMismatch: false,
					throwOnCompilationError: false,
				}

				const result = compileContracts(solc, sources, options, mockLogger)
				const source = result.compilationResult['Contract.sol']
				assert(source, 'Source not found')

				const contract = source.getContract()
				expect(contract).toBeDefined()
				expect(contract.name).toBe('TestContract')
			})
		})

		describe('error handling', () => {
			it('should throw when missing required compilation output fields', () => {
				const sources = {
					'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
				}

				const options: ValidatedCompileBaseOptions = {
					language: 'Solidity',
					compilationOutput: ['abi'], // Missing bytecode and deployedBytecode
					hardfork: 'cancun',
					solcVersion: '0.8.28',
					throwOnVersionMismatch: false,
					throwOnCompilationError: false,
				}

				const result = compileContracts(solc, sources, options, mockLogger)
				const source = result.compilationResult['Contract.sol']
				assert(source, 'Source not found')

				expect(() => source.getContract()).toThrow(CompilerOutputError)
				expect(() => source.getContract()).toThrow(/missing bytecode, deployedBytecode/)

				try {
					source.getContract()
				} catch (error) {
					if (error instanceof CompilerOutputError) {
						expect(error.meta?.code).toBe('missing_compilation_output')
					} else {
						throw error
					}
				}
			})

			it('should throw when missing abi', () => {
				const sources = {
					'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
				}

				const options: ValidatedCompileBaseOptions = {
					language: 'Solidity',
					compilationOutput: ['bytecode', 'deployedBytecode'], // Missing abi
					hardfork: 'cancun',
					solcVersion: '0.8.28',
					throwOnVersionMismatch: false,
					throwOnCompilationError: false,
				}

				const result = compileContracts(solc, sources, options, mockLogger)
				const source = result.compilationResult['Contract.sol']
				assert(source, 'Source not found')

				expect(() => source.getContract()).toThrow(CompilerOutputError)
				expect(() => source.getContract()).toThrow(/missing abi/)

				try {
					source.getContract()
				} catch (error) {
					if (error instanceof CompilerOutputError) {
						expect(error.meta?.code).toBe('missing_compilation_output')
					} else {
						throw error
					}
				}
			})

			it('should throw when no contracts exist in source', () => {
				const sources = {
					'Empty.sol': 'pragma solidity ^0.8.0;',
				}

				const options: ValidatedCompileBaseOptions = {
					language: 'Solidity',
					compilationOutput: ['abi', 'bytecode', 'deployedBytecode'],
					hardfork: 'cancun',
					solcVersion: '0.8.28',
					throwOnVersionMismatch: false,
					throwOnCompilationError: false,
				}

				const result = compileContracts(solc, sources, options, mockLogger)
				const source = result.compilationResult['Empty.sol']
				assert(source, 'Source not found')

				expect(() => source.getContract()).toThrow(CompilerOutputError)
				expect(() => source.getContract()).toThrow(/No contracts found in source/)

				try {
					source.getContract()
				} catch (error) {
					if (error instanceof CompilerOutputError) {
						expect(error.meta?.code).toBe('no_contracts')
						expect(error.meta?.sourcePath).toBe('Empty.sol')
					} else {
						throw error
					}
				}
			})

			it('should throw when contract name not specified for multiple contracts', () => {
				const sources = {
					'Multi.sol': 'pragma solidity ^0.8.0; contract A {} contract B {}',
				}

				const options: ValidatedCompileBaseOptions = {
					language: 'Solidity',
					compilationOutput: ['abi', 'bytecode', 'deployedBytecode'],
					hardfork: 'cancun',
					solcVersion: '0.8.28',
					throwOnVersionMismatch: false,
					throwOnCompilationError: false,
				}

				const result = compileContracts(solc, sources, options, mockLogger)
				const source = result.compilationResult['Multi.sol']
				assert(source, 'Source not found')

				expect(() => source.getContract()).toThrow(CompilerOutputError)
				expect(() => source.getContract()).toThrow(/Multiple contracts found/)

				try {
					source.getContract()
				} catch (error) {
					if (error instanceof CompilerOutputError) {
						expect(error.meta?.code).toBe('ambiguous_contract')
						expect(error.meta?.sourcePath).toBe('Multi.sol')
						expect(error.meta?.availableContracts).toEqual(['A', 'B'])
					} else {
						throw error
					}
				}
			})

			it('should throw when specified contract name not found', () => {
				const sources = {
					'Contract.sol': 'pragma solidity ^0.8.0; contract TestContract {}',
				}

				const options: ValidatedCompileBaseOptions = {
					language: 'Solidity',
					compilationOutput: ['abi', 'bytecode', 'deployedBytecode'],
					hardfork: 'cancun',
					solcVersion: '0.8.28',
					throwOnVersionMismatch: false,
					throwOnCompilationError: false,
				}

				const result = compileContracts(solc, sources, options, mockLogger)
				const source = result.compilationResult['Contract.sol']
				assert(source, 'Source not found')

				expect(() => source.getContract('NonExistent')).toThrow(CompilerOutputError)
				expect(() => source.getContract('NonExistent')).toThrow(/Contract "NonExistent" not found/)

				try {
					source.getContract('NonExistent')
				} catch (error) {
					if (error instanceof CompilerOutputError) {
						expect(error.meta?.code).toBe('contract_not_found')
						expect(error.meta?.contractName).toBe('NonExistent')
						expect(error.meta?.sourcePath).toBe('Contract.sol')
						expect(error.meta?.availableContracts).toEqual(['TestContract'])
					} else {
						throw error
					}
				}
			})
		})
	})
})
