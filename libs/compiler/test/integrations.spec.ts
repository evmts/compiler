import { beforeAll, describe, test } from 'bun:test'
import { whatsabi } from '@shazow/whatsabi'

// Various integration examples for shadowing deployed contracts on a 1:1 matching mainnet environment
describe.skip('Integrations', () => {
	// The Ast that ships with the compiler pairs well with tools such as whatsabi to fetch verified contracts,
	// as we can manipulate a contract AST as long as we are provided an original source code or parsed AST.
	describe('Use with whatsabi', async () => {
		// An obvious use case for instance is to:
		// 1. fetch a verified contract;
		// 2. manipulate its AST to expose internal variables/functions;
		// 3. compile the instrumented AST;
		// 4. use the instrumented bytecode in the context of the original contract to extend its functionality
		let _whatsabiResult
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
			const _result = loader.getContract('')
		})

		test.todo('can create a Contract instance out of a verified contract', () => {})
		test.todo('can create an Ast instance out of a verified contract', () => {})
		test.todo('instrument ast + compile again + replace bytecode and interact', () => {})
	})

	describe('Various use cases', () => {
		test.todo('ERC721A: expose packed ownership via shadow getter', () => {
			// Outline:
			// 1. Clone verified ERC721A source (e.g. Azuki’s implementation mirrored on Sourcify).
			// 2. Inject __debugOwnership(uint256) that calls the private _packedOwnershipOf(tokenId)
			//    helper and unpacks owner/startTimestamp/burned/extraData from the packed word.
			// 3. Compile with tevm compiler, patch live fork bytecode via setCode, then read using viem.
			// Why shadowing:
			// - Public ABI only gives ownerOf; raw packed word requires brittle slot math against
			//   mapping(uint256 => uint256) _packedOwnerships that varies across versions/proxies.
			// - Shadow getter guarantees layout-correct decode across ERC721A forks without guessing.
			// Reference viem usage:
			// const { result } = await client.readContract({ abi, functionName: '__debugOwnership', args: [tokenId] })
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
