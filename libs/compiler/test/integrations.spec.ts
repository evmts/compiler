import { beforeAll, describe, expect, spyOn, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { whatsabi } from '@shazow/whatsabi'
import { Ast, Compiler, Contract } from '../build/index.js'
import {
	encodeUintWords,
	etherToWei,
	padAddress,
	setupCompound,
	setupErc721A,
	setupSeaport,
	setupUniswapV3,
	setupUniswapV4,
} from './integration.setup.js'

// Various integration examples for shadowing contracts and enable better workflows on a 1:1 matching mainnet environment
describe('Integrations', () => {
	// The Ast that ships with the compiler pairs well with tools such as whatsabi to fetch verified contracts,
	// as we can manipulate a contract AST as long as we are provided an original source code or parsed AST.
	describe('Use with whatsabi', () => {
		const address = '0x0000000000000000000000000000000000000001'
		const sourceName = 'Counter.sol'
		const source = readFileSync(join(__dirname, 'fixtures', 'contracts', sourceName), 'utf8')
		const metadata = {
			compiler: { version: '0.8.30+commit.73712a01' },
			language: 'Solidity',
			output: {
				abi: [{ type: 'function', name: 'count', inputs: [], outputs: [{ type: 'uint256' }], stateMutability: 'view' }],
			},
			settings: { compilationTarget: { [sourceName]: 'Counter' }, optimizer: { enabled: false, runs: 200 } },
		}
		let whatsabiResult: whatsabi.loaders.ContractResult
		beforeAll(async () => {
			const fetch = spyOn(globalThis, 'fetch').mockResolvedValue(
				Response.json({
					files: [
						{ name: 'metadata.json', content: JSON.stringify(metadata) },
						{ name: sourceName, path: sourceName, content: source },
					],
				}),
			)
			try {
				const loader = new whatsabi.loaders.MultiABILoader([new whatsabi.loaders.SourcifyABILoader({ chainId: 1 })])
				whatsabiResult = await loader.getContract(address)
				expect(whatsabiResult.ok).toBe(true)
				expect(fetch.mock.calls.map(([url]) => url)).toEqual([`https://sourcify.dev/server/files/1/${address}`])
			} finally {
				fetch.mockRestore()
			}
		})

		test('Create a Contract instance out of a verified contract', () => {
			const { name, abi } = whatsabiResult
			if (!name || !abi) throw new Error('Sourcify metadata must contain a contract name and ABI')
			const contract = new Contract({ name, abi }).withAddress(address)
			expect(contract.address).toBe(address)
			expect(contract.name).toBe('Counter')
			expect(contract.abi).toEqual(metadata.output.abi)
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
				instrumentedContract: whatsabiResult.name ?? undefined,
			})
			const sources = await whatsabiResult.getSources?.()
			const verifiedSource = sources?.find((file) => file.path === sourceName)
			if (!verifiedSource) throw new Error(`Missing ${sourceName} in the Sourcify response`)
			ast.fromSource(verifiedSource.content)

			const sourceUnit = ast.sourceUnit()
			const contracts = sourceUnit.nodes.filter((node) => node.nodeType === 'ContractDefinition')
			expect(contracts.map((c) => c.name)).toEqual(['Counter'])
		})
	})

	describe('Various use cases', () => {
		test('Uniswap v3: decode TickBitmap word in-contract', async () => {
			const { client, contract, source, tickSpacing } = await setupUniswapV3()

			const bitmapDecoderShadow = `
				function decodeTickBitmap(int16 wordIndex, int24 spacing) external view returns (int24[] memory ticks) {
					require(spacing > 0, "TickSpacingZero");
					uint256 word = _tickBitmap[wordIndex];
					uint256 population;
					for (uint16 bit = 0; bit < 256; bit++) {
						if ((word & (uint256(1) << bit)) != 0) {
							population++;
						}
					}
					ticks = new int24[](population);
					uint256 cursor;
					for (uint16 bit = 0; bit < 256; bit++) {
						if ((word & (uint256(1) << bit)) != 0) {
							int256 tick = (int256(wordIndex) * 256 + int256(uint256(bit))) * int256(spacing);
							ticks[cursor] = int24(tick);
							cursor++;
						}
					}
					return ticks;
				}
			`

			// Shadow the pool with an on-chain bitmap decoder mirroring the off-chain bit math
			const output = new Ast({ instrumentedContract: 'UniswapV3PoolMock' })
				.fromSource(source)
				.injectShadow(bitmapDecoderShadow)
				.compile()
			if (output.hasCompilerErrors()) {
				throw new Error(`Failed to compile: ${output.errors.map((e) => e.formattedMessage).join(', ')}`)
			}
			const instrumentedContract = output.artifact.contracts['UniswapV3PoolMock']

			// Call into the shadow helper and present numeric ticks for the expectations
			const decode = async (wordIndex: number) =>
				client
					.tevmContract({
						to: contract.address,
						abi: instrumentedContract.abi!,
						deployedBytecode: instrumentedContract.deployedBytecode!.hex,
						functionName: 'decodeTickBitmap',
						args: [wordIndex, tickSpacing],
					})
					.then((response) => response.data)
			expect(await decode(-1)).toEqual([-120])
			expect(await decode(0)).toEqual([0, 60])
		})

		test('Uniswap v4: override hook dispatch for fork testing', async () => {
			const { client, poolId, poolManager, defaultHooks, overrideHooks, source, swapData } = await setupUniswapV4()

			const dispatchShadow = `
				function dispatchSwapWithHookOverride(bytes32 poolId, address overrideHook, bytes calldata data)
					external
					returns (address hook, bytes memory result)
				{
					hook = overrideHook != address(0) ? overrideHook : pools[poolId].hooks;
					if (hook == address(0)) {
						lastHookInvoked = address(0);
						return (address(0), "");
					}
					lastHookInvoked = hook;
					result = IHooks(hook).beforeSwap(msg.sender, data);
				}
			`

			// Expose a dev-only dispatcher that can override the hook target
			const output = new Ast({ instrumentedContract: 'PoolManagerMock' })
				.fromSource(source)
				.injectShadow(dispatchShadow)
				.compile()
			if (output.hasCompilerErrors()) {
				throw new Error(`Failed to compile: ${output.errors.map((e) => e.formattedMessage).join(', ')}`)
			}
			const instrumentedPoolManager = output.artifact.contracts['PoolManagerMock']

			const dispatch = async (overrideHook = `0x${'0'.repeat(40)}`) =>
				client.tevmContract({
					to: poolManager.address,
					abi: instrumentedPoolManager.abi!,
					deployedBytecode: instrumentedPoolManager.deployedBytecode!.hex,
					functionName: 'dispatchSwapWithHookOverride',
					args: [poolId, overrideHook, swapData],
					addToBlockchain: true,
				})

			// First call uses the pool's registered hook
			const initialDispatch = await dispatch()

			// Confirm the pool still reports the original hook in storage
			const { data: hookFromStorage } = await client.tevmContract({
				to: poolManager.address,
				abi: instrumentedPoolManager.abi!,
				deployedBytecode: instrumentedPoolManager.deployedBytecode!.hex,
				functionName: 'poolHooks',
				args: [poolId],
			})
			expect((hookFromStorage as string).toLowerCase()).toBe(defaultHooks.address.toLowerCase())

			// First call uses the pool's registered hook in storage
			const [initialHook, initialResult] = initialDispatch.data as [string, string]
			expect([initialHook.toLowerCase(), initialResult]).toEqual([defaultHooks.address.toLowerCase(), swapData])

			// Subsequent call forces the override hook via the injected helper
			const overriddenDispatch = await dispatch(overrideHooks.address)
			const [overrideHook, overrideResult] = overriddenDispatch.data as [string, string]
			expect([overrideHook.toLowerCase(), overrideResult]).toEqual([overrideHooks.address.toLowerCase(), swapData])

			const { data: hookAfterOverride } = await client.tevmContract({
				to: poolManager.address,
				abi: instrumentedPoolManager.abi!,
				deployedBytecode: instrumentedPoolManager.deployedBytecode!.hex,
				functionName: 'poolHooks',
				args: [poolId],
			})
			expect((hookAfterOverride as string).toLowerCase()).toBe(defaultHooks.address.toLowerCase())
		})

		test('Compound v2: shadow exchange-rate invariant checker', async () => {
			const { client, contract, source } = await setupCompound()

			const invariantShadow = `
				error BrokenInvariant();
				function exchangeRateInvariant() internal view {
					uint256 supply = totalSupply();
					if (supply == 0) revert BrokenInvariant();
					uint256 expected = (getCash() + totalBorrows - totalReserves) * 1e18 / supply;
					uint256 stored = exchangeRateStoredInternal();
					if (expected != stored) revert BrokenInvariant();
				}
			`

			// Publish the borrow function wrapped with the exchange-rate invariant
			const output = new Ast({ instrumentedContract: 'CErc20DelegateMock' })
				.fromSource(source)
				.injectShadow(invariantShadow)
				.injectShadowAtEdges('borrow', {
					before: 'exchangeRateInvariant();',
					after: 'exchangeRateInvariant();',
				})
				.compile()
			if (output.hasCompilerErrors()) {
				throw new Error(`Failed to compile: ${output.errors.map((e) => e.formattedMessage).join(', ')}`)
			}
			const instrumentedContract = output.artifact.contracts['CErc20DelegateMock']

			// A dust borrow accumulates less than one wei of reserves (after scaling) and passes the invariant
			await client.tevmContract({
				to: contract.address,
				abi: instrumentedContract.abi!,
				deployedBytecode: instrumentedContract.deployedBytecode!.hex,
				functionName: 'borrow',
				args: [1n],
				addToBlockchain: true,
			})

			// The contract has an obvious intentional bug that truncates the reserves to zero, which gets the
			// exchange rate to ignore the newly-accrued reserves, and breaks the invariant
			expect(
				client.tevmContract({
					to: contract.address,
					abi: instrumentedContract.abi!,
					deployedBytecode: instrumentedContract.deployedBytecode!.hex,
					functionName: 'borrow',
					args: [etherToWei(0.4)],
					addToBlockchain: true,
				}),
			).rejects.toThrowError('Error: BrokenInvariant()')
		})

		test('Seaport: shadow wrapper emitting ShadowSale summaries', async () => {
			const { client, contract, source, order, fulfiller } = await setupSeaport()

			const shadowSale = `
				event ShadowSale(
					address indexed offerer,
					address indexed fulfiller,
					uint256 offererDebited,
					uint256 fulfillerCredited,
					uint256 totalConsideration
				);

				function fulfillOrderWithShadow(Order memory params)
					external
					returns (uint256 offererDebited, uint256 fulfillerCredited, uint256[] memory considerationCredits)
				{
					uint256 beforeOfferer = balances[params.offerer];
					uint256 beforeFulfiller = balances[params.fulfiller];
					considerationCredits = new uint256[](params.consideration.length);
					uint256[] memory snapshots = new uint256[](params.consideration.length);
					for (uint256 i = 0; i < params.consideration.length; i++) {
						snapshots[i] = balances[params.consideration[i].recipient];
					}

					require(fulfillOrder(params), "FULFILL_FAILED");

					offererDebited = beforeOfferer - balances[params.offerer];
					fulfillerCredited = balances[params.fulfiller] - beforeFulfiller;
					uint256 mintedToFulfiller = params.offer.amount;
					uint256 totalConsideration;
					for (uint256 i = 0; i < params.consideration.length; i++) {
						address recipient = params.consideration[i].recipient;
						uint256 delta = balances[recipient] - snapshots[i];
						if (recipient == params.fulfiller && mintedToFulfiller != 0) {
							considerationCredits[i] = delta - mintedToFulfiller;
							mintedToFulfiller = 0;
						} else {
							considerationCredits[i] = delta;
						}
						totalConsideration += considerationCredits[i];
					}

					emit ShadowSale(params.offerer, params.fulfiller, offererDebited, fulfillerCredited, totalConsideration);
				}
			`

			// Wrap fulfillOrder with a summary emitter that mirrors Seaport flows
			const output = new Ast({ instrumentedContract: 'SeaportMock' })
				.fromSource(source)
				.injectShadow(shadowSale)
				.compile()
			if (output.hasCompilerErrors()) {
				throw new Error(`Failed to compile: ${output.errors.map((e) => e.formattedMessage).join(', ')}`)
			}
			const instrumentedContract = output.artifact.contracts['SeaportMock']

			const response = await client.tevmContract({
				to: contract.address,
				abi: instrumentedContract.abi!,
				deployedBytecode: instrumentedContract.deployedBytecode!.hex,
				functionName: 'fulfillOrderWithShadow',
				args: [order],
				addToBlockchain: true,
			})

			const [offererDebited, fulfillerCredited, considerationCredits] = response.data as [bigint, bigint, bigint[]]

			expect(offererDebited).toBe(order.offer.amount)
			const fulfillerConsideration = order.consideration
				.filter((item) => item.recipient === fulfiller)
				.reduce((acc, item) => acc + item.amount, 0n)
			// Fulfiller receives both the listed item and any direct consideration
			expect(fulfillerCredited).toBe(order.offer.amount + fulfillerConsideration)
			expect(considerationCredits).toEqual(order.consideration.map((item) => item.amount))

			const shadowSaleTopic = '0x963207103f08cdcf5c0d2473aba5d66d197542d11ef0f71e99070632c22fe8df'
			const log = response.logs?.find((entry) => entry.topics[0] === shadowSaleTopic)

			expect(log).toMatchObject({
				address: contract.address,
				topics: [shadowSaleTopic, padAddress(order.offerer), padAddress(fulfiller)],
				data: encodeUintWords(
					offererDebited,
					fulfillerCredited,
					considerationCredits.reduce((sum, value) => sum + value, 0n),
				),
			})
		})
	})

	test('ERC721A: expose packed address data analytics', async () => {
		const { callerAddress, client, contract, source } = await setupErc721A()

		const analyticsShadow = `
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
		`

		// Instrument the ERC721A mock with an analytics surface mirroring packed address data
		const ast = new Ast({ instrumentedContract: 'ERC721AMock' })
			.fromSource(source)
			.injectShadow(analyticsShadow)
			.validate()

		// Compile the AST (this will reuse the cached output from validation here)
		const output = ast.compile()
		if (output.hasCompilerErrors()) {
			throw new Error(`Failed to compile: ${output.errors.map((e) => e.formattedMessage).join(', ')}`)
		}

		const instrumentedContract = output.artifact.contracts['ERC721AMock']
		// Call the original contract using the intrumented bytecode
		// Meaning execute the instrumented (shadowed) contract in the context of the original contract
		const res = await client.tevmContract({
			to: contract.address,
			abi: instrumentedContract.abi!,
			deployedBytecode: instrumentedContract.deployedBytecode!.hex,
			functionName: 'addressAnalytics',
			args: [callerAddress],
		})
		expect(res.data).toMatchObject({ balance: 1n, minted: 1n, burned: 0n, aux: 0n })
	})
})
