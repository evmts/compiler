import { beforeAll, describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { whatsabi } from '@shazow/whatsabi'
import { createMemoryClient, MemoryClient } from '@tevm/memory-client'
import { Ast, Compiler, Contract, FullyDefinedMap } from '../build/index.js'

// Various integration examples for shadowing contracts and enable better workflows on a 1:1 matching mainnet environment
describe('Integrations', () => {
	// The Ast that ships with the compiler pairs well with tools such as whatsabi to fetch verified contracts,
	// as we can manipulate a contract AST as long as we are provided an original source code or parsed AST.
	describe('Use with whatsabi', async () => {
		const BEACON_CONTRACT_ADDRESS = '0x00000000219ab540356cBB839Cbe05303d7705Fa'
		// An obvious use case for instance is to:
		// 1. fetch a verified contract;
		// 2. manipulate its AST to expose internal variables/functions;
		// 3. compile the instrumented AST;
		// 4. use the instrumented bytecode in the context of the original contract to extend its functionality
		let whatsabiResult: whatsabi.loaders.ContractResult
		beforeAll(async () => {
			// See https://shazow.github.io/whatsabi/ for using whatsabi
			const loader = new whatsabi.loaders.MultiABILoader([
				new whatsabi.loaders.SourcifyABILoader({ chainId: 1 }),
				// Add fallbacks for Etherscan and Blockscout to maximize coverage
				// new whatsabi.loaders.EtherscanV2ABILoader({
				// 	apiKey: '...', // Replace the value with your Etherscan API key
				// }),
				// new whatsabi.loaders.BlockscoutABILoader({
				// 	apiKey: '...', // Replace the value with your Blockscout API key
				// }),
			])
			const result = await loader.getContract(BEACON_CONTRACT_ADDRESS)
			if (!result.ok) throw new Error('Failed to load contract from Sourcify')
			whatsabiResult = result
		})

		test('Create a Contract instance out of a verified contract', () => {
			const contract = new Contract({ name: whatsabiResult.name, ...whatsabiResult.loaderResult.output }).withAddress(
				BEACON_CONTRACT_ADDRESS,
			)
			expect(contract.address).toBe(BEACON_CONTRACT_ADDRESS)
			expect(contract.name).toBe(whatsabiResult.name)
			expect(contract.abi).toEqual(whatsabiResult.loaderResult.output.abi)
		})
		test('Create an Ast instance out of a verified contract', async () => {
			const solcSettings = whatsabiResult.loaderResult.settings
			const compilerVersion = whatsabiResult.compilerVersion?.split('+')[0]
			if (compilerVersion && !Compiler.isSolcVersionInstalled(compilerVersion)) {
				await Compiler.installSolcVersion(compilerVersion)
			}

			const ast = new Ast({
				solcLanguage: whatsabiResult.loaderResult.language === 'Solidity' ? 'solidity' : 'yul',
				solcVersion: compilerVersion,
				solcSettings: {
					evmVersion: solcSettings.evmVersion,
					optimizer: solcSettings.optimizer,
					libraries: solcSettings.libraries,
					remappings: solcSettings.remappings,
				},
				instrumentedContract: 'DepositContract',
			}).fromSource(whatsabiResult.loaderResult.sources['deposit_contract.sol'].content)

			const sourceUnit = ast.sourceUnit()
			const contracts = sourceUnit.nodes.filter((node) => node.nodeType === 'ContractDefinition')
			expect(contracts.map((c) => c.name)).toEqual(['IDepositContract', 'ERC165', 'DepositContract'])
		})
	})

	describe('Various use cases', () => {
		let client: MemoryClient
		const erc721APath = join(__dirname, 'fixtures', 'contracts', 'ERC721A.sol')
		let erc721AContract: Contract<string, FullyDefinedMap>
		const callerAddress = `0x${'1'.repeat(40)}` as const

		beforeAll(async () => {
			// If we wanted to actually work with a fork and an onchain contract, we would create a client
			// in fork mode and use the actual contract address; we don't here for the sake of simplicity.
			// client = createMemoryClient({ fork: { transport: http('mainnet.rpc.url') }})
			client = createMemoryClient() as unknown as MemoryClient

			// The following are purely test setup; on a fork we would want to use the actual fork state instead
			// or this in-memory test state
			const compiler = new Compiler({ solcVersion: '0.8.30' })
			const output = compiler.compileFiles([erc721APath])
			if (output.hasCompilerErrors()) {
				throw new Error(`Failed to compile: ${output.errors.map((e) => e.formattedMessage).join(', ')}`)
			}

			erc721AContract = output.artifacts[erc721APath].contracts['ERC721AMock'].withAddress(`0x${'a'.repeat(40)}`)
			await client.tevmSetAccount({
				address: erc721AContract.address!,
				deployedBytecode: erc721AContract.deployedBytecode!.hex,
			})
			await client.tevmContract({
				to: erc721AContract.address!,
				abi: erc721AContract.abi!,
				functionName: 'mint',
				args: [callerAddress, 1],
				addToBlockchain: true,
			})
		})

		test.only('ERC721A: expose packed address data analytics', async () => {
			const ast = new Ast({
				solcVersion: '0.8.30',
				instrumentedContract: 'ERC721AMock',
			})
				.fromSource(readFileSync(erc721APath, 'utf8'))
				// Add the shadow function that creates the analytics
				.injectShadow(`
			struct Analytics {
				uint64 balance;
				uint64 minted;
				uint64 burned;
				uint64 aux;
			}

			function addressAnalytics(address owner) external view returns (Analytics memory) {
				return Analytics(
				  uint64(balanceOf(owner)),
				  uint64(_numberMinted(owner)),
				  uint64(_numberBurned(owner)),
				  uint64(_getAux(owner))
				);
			}
				`)
				// Validate to make sure the AST is valid (this will compile it internally)
				.validate()

			// Compile the AST (this will reuse the cached output from validation here)
			const output = ast.compile()
			if (output.hasCompilerErrors()) {
				throw new Error(`Failed to compile: ${output.errors.map((e) => e.formattedMessage).join(', ')}`)
			}

			const instrumentedContract = output.artifact.contracts['ERC721AMock'].withAddress(erc721AContract.address)
			// Call the original contract using the intrumented bytecode
			// Meaning execute the instrumented (shadowed) contract in the context of the original contract
			const res = await client.tevmContract({
				to: instrumentedContract.address!,
				abi: instrumentedContract.abi!,
				deployedBytecode: instrumentedContract.deployedBytecode!.hex,
				functionName: 'addressAnalytics',
				args: [callerAddress],
			})
			expect(res.data).toMatchObject({ balance: 1n, minted: 1n, burned: 0n, aux: 0n })
		})

		test.todo('Uniswap v3: decode TickBitmap word in-contract', () => {
			// Outline:
			// 1. Patch UniswapV3Pool (or a thin reader) with __decodeTickBitmap(int16,uint24) that
			//    reads tickBitmap(wordIndex), counts set bits, allocates int24[] ticks, iterates bits,
			//    and maps bit index → tick via (wordIndex * 256 + bitIndex) * tickSpacing.
			// 2. Compile + deploy patched bytecode on fork, call helper from viem to recover ticks.
			// Why shadowing:
			// - ABI exposes tickBitmap(wordIndex) but not enumeration; off-chain scanning requires
			//   guessing non-zero words and manual bit math, creating many RPC calls and easy mistakes.
			// - On-chain decoder returns canonical tick list in one call per word.
			// Reference viem usage:
			// const { result: ticks } = await client.readContract({ functionName: '__decodeTickBitmap', args: [wordIndex, spacing] })
		})
		test.todo('Uniswap v4: override hook dispatch for fork testing', () => {
			// Outline:
			// 1. Fork PoolManager, add storage slot address __testHook and setter __setTestHook(address).
			// 2. In hook dispatch path (beforeSwap/afterSwap, liquidity callbacks, etc.), use
			//    hookToCall = __testHook != address(0) ? __testHook : address(key.hooks).
			// 3. Compile patched PoolManager, patch forked bytecode, set custom hook via viem,
			//    run swaps to exercise custom hook against unchanged pool state.
			// Why shadowing:
			// - PoolKey (currency0, currency1, fee, tickSpacing, hooks) is hashed into PoolId at init,
			//   so hook address is immutable; ABI offers no way to attach a new hook to an existing pool.
			// - Shadow override enables testing hooks on real pool liquidity without reinitializing.
			// Reference viem usage:
			// await client.writeContract({ functionName: '__setTestHook', args: ['0xYourHook'] })
		})
		test.todo('Compound v2: shadow exchange-rate invariant checker', () => {
			// Outline:
			// 1. Add __checkExchangeRateInvariant() view that reads getCash(), totalBorrows(),
			//    totalReserves(), totalSupply(), mirrors exchangeRateStoredInternal() math, and
			//    returns (ok, expected, stored).
			// 2. Wrap relevant functions with a pre & post invariant check by calling this function.
			// 3. Use tevm compiler to rebuild instrumented cToken, patch fork, call helper from viem.
			// Why shadowing (despite ABI parity):
			// - Public views allow reconstruction, but shadow method guarantees identical rounding/
			//   scaling logic and offers one-call forensic output with explicit pass/fail context.
			// Reference viem usage:
			// const { result } = await client.readContract({ functionName: '__checkExchangeRateInvariant' })
		})
		test.todo('Seaport: shadow wrapper emitting ShadowSale summaries', () => {
			// Outline:
			// 1. Wrap fulfillOrder / fulfillBasicOrder with a dev-only layer that snapshots balances/
			//    transfers pre/post-call and emits a ShadowSale event summarizing consideration flows.
			// 2. Compile patched Seaport contract, replace bytecode on fork, run marketplace fills,
			//    consume ShadowSale events in tests.
			// Why shadowing:
			// - ABI emits granular events; aggregating actual fills (multi-item bundles, fees, royalties)
			//   off-chain is error-prone and requires stitching numerous logs.
			// - Wrapper delivers canonical “who paid/received what” event for analytics and debugging.
			// Reference viem usage:
			// await client.writeContract({ functionName: 'fulfillOrder', args: [...] }) // event logs contain ShadowSale summary
		})
	})
})
