/**
 * Test harness utilities used by the documentation-style integration specs.
 * Each helper compiles a single Solidity fixture, seeds a dedicated in-memory
 * client, and returns the minimal state required for the instrumentation tests.
 */
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { createMemoryClient, MemoryClient } from '@tevm/memory-client'
import { Compiler } from '../build/index.js'

const fixturesRoot = join(__dirname, 'fixtures', 'contracts')
export const etherToWei = (ether: number) => BigInt(ether * 10 ** 18)
export const padAddress = (value: `0x${string}`) => `0x${value.slice(2).padStart(64, '0')}`
export const encodeUintWords = (...values: bigint[]) =>
	`0x${values.map((value) => value.toString(16).padStart(64, '0')).join('')}`

/** Prepare an ERC721A fixture with a minted token to shadow. */
export const setupErc721A = async () => {
	const client = createMemoryClient()
	const path = join(fixturesRoot, 'ERC721A.sol')
	const compiler = new Compiler()
	const output = compiler.compileFiles([path])
	if (output.hasCompilerErrors()) {
		throw new Error(`Failed to compile ${path}: ${output.errors.map((e) => e.formattedMessage).join(', ')}`)
	}
	const contract = output.artifacts[path].contracts['ERC721AMock'].withAddress(`0x${'a'.repeat(40)}`)
	const callerAddress = `0x${'1'.repeat(40)}` as const

	await client.tevmSetAccount({
		address: contract.address!,
		deployedBytecode: contract.deployedBytecode!.hex,
	})

	await client.tevmContract({
		to: contract.address!,
		abi: contract.abi!,
		functionName: 'mint',
		args: [callerAddress, 1],
		addToBlockchain: true,
	})

	return {
		callerAddress,
		client: client as unknown as MemoryClient,
		contract,
		source: readFileSync(path, 'utf8'),
	}
}

/** Prepare a Uniswap v3 bitmap fixture with seeded ticks for decoding. */
export const setupUniswapV3 = async () => {
	const client = createMemoryClient()
	const path = join(fixturesRoot, 'UniswapV3Pool.sol')
	const compiler = new Compiler()
	const output = compiler.compileFiles([path])
	if (output.hasCompilerErrors()) {
		throw new Error(`Failed to compile ${path}: ${output.errors.map((e) => e.formattedMessage).join(', ')}`)
	}
	const contract = output.artifacts[path].contracts['UniswapV3PoolMock'].withAddress(`0x${'b'.repeat(40)}`)
	const tickSpacing = 60
	const seededTicks = [-120, 0, 60] as const

	await client.tevmSetAccount({
		address: contract.address!,
		deployedBytecode: contract.deployedBytecode!.hex,
	})

	for (const tick of seededTicks) {
		await client.tevmContract({
			to: contract.address!,
			abi: contract.abi!,
			functionName: 'flipTick',
			args: [tick, tickSpacing],
			addToBlockchain: true,
		})
	}

	return {
		client: client as unknown as MemoryClient,
		contract,
		source: readFileSync(path, 'utf8'),
		tickSpacing,
		seededTicks,
	}
}

/** Prepare a Uniswap v4 pool manager with default and override hook contracts deployed. */
export const setupUniswapV4 = async () => {
	const client = createMemoryClient()
	const path = join(fixturesRoot, 'UniswapV4PoolManager.sol')
	const compiler = new Compiler()
	const output = compiler.compileFiles([path])
	if (output.hasCompilerErrors()) {
		throw new Error(`Failed to compile ${path}: ${output.errors.map((e) => e.formattedMessage).join(', ')}`)
	}
	const poolManager = output.artifacts[path].contracts['PoolManagerMock'].withAddress(`0x${'c'.repeat(40)}`)
	const defaultHooks = output.artifacts[path].contracts['LoggingHooksMock'].withAddress(`0x${'d'.repeat(40)}`)
	const overrideHooks = output.artifacts[path].contracts['LoggingHooksMock'].withAddress(`0x${'e'.repeat(40)}`)

	for (const contract of [poolManager, defaultHooks, overrideHooks]) {
		await client.tevmSetAccount({
			address: contract.address!,
			deployedBytecode: contract.deployedBytecode!.hex,
		})
	}

	const poolKey = {
		currency0: `0x${'1'.repeat(40)}`,
		currency1: `0x${'2'.repeat(40)}`,
		fee: 3000,
		tickSpacing: 60,
		hooks: defaultHooks.address!,
	}

	await client.tevmContract({
		to: poolManager.address!,
		abi: poolManager.abi!,
		functionName: 'registerPool',
		args: [poolKey],
		addToBlockchain: true,
	})

	const poolIdResult = await client.tevmContract({
		to: poolManager.address!,
		abi: poolManager.abi!,
		functionName: 'lastRegisteredPoolId',
	})
	const poolId = poolIdResult.data as `0x${string}`

	return {
		client: client as unknown as MemoryClient,
		poolId,
		poolKey,
		poolManager,
		defaultHooks,
		overrideHooks,
		source: readFileSync(path, 'utf8'),
		swapData: `0x${'12'.repeat(16)}` as const,
	}
}

/** Prepare a Compound CErc20 delegate with representative accounting numbers. */
export const setupCompound = async () => {
	const client = createMemoryClient()
	const path = join(fixturesRoot, 'CompoundCErc20.sol')
	const compiler = new Compiler()
	const output = compiler.compileFiles([path])
	if (output.hasCompilerErrors()) {
		throw new Error(`Failed to compile ${path}: ${output.errors.map((e) => e.formattedMessage).join(', ')}`)
	}
	const contract = output.artifacts[path].contracts['CErc20DelegateMock'].withAddress(`0x${'f'.repeat(40)}`)

	await client.tevmSetAccount({
		address: contract.address!,
		deployedBytecode: contract.deployedBytecode!.hex,
	})

	const totals = {
		cash: etherToWei(1),
		borrows: etherToWei(0.6),
		reserves: 0n,
		supply: etherToWei(0.005),
	}

	await client.tevmContract({
		to: contract.address!,
		abi: contract.abi!,
		functionName: 'setTotals',
		args: [totals.cash, totals.borrows, totals.reserves, totals.supply],
		addToBlockchain: true,
	})

	await client.tevmContract({
		to: contract.address!,
		abi: contract.abi!,
		functionName: 'setAccounting',
		args: [etherToWei(0.1), 123_456n, etherToWei(1)],
		addToBlockchain: true,
	})

	return {
		client: client as unknown as MemoryClient,
		contract,
		source: readFileSync(path, 'utf8'),
		totals,
	}
}

/** Prepare a Seaport-like matcher with pre-seeded balances and a simple order. */
export const setupSeaport = async () => {
	const client = createMemoryClient()
	const path = join(fixturesRoot, 'Seaport.sol')
	const compiler = new Compiler()
	const output = compiler.compileFiles([path])
	if (output.hasCompilerErrors()) {
		throw new Error(`Failed to compile ${path}: ${output.errors.map((e) => e.formattedMessage).join(', ')}`)
	}
	const contract = output.artifacts[path].contracts['SeaportMock'].withAddress(`0x${'9'.repeat(40)}`)

	await client.tevmSetAccount({
		address: contract.address!,
		deployedBytecode: contract.deployedBytecode!.hex,
	})

	const offerer = `0x${'3'.repeat(40)}` as const
	const fulfiller = `0x${'4'.repeat(40)}` as const
	const royaltyRecipient = `0x${'5'.repeat(40)}` as const

	await client.tevmContract({
		to: contract.address!,
		abi: contract.abi!,
		functionName: 'seedBalance',
		args: [offerer, etherToWei(1)],
		addToBlockchain: true,
	})

	const order = {
		offerer,
		fulfiller,
		offer: { token: `0x${'6'.repeat(40)}`, amount: etherToWei(0.5) },
		consideration: [
			{ recipient: fulfiller, amount: etherToWei(0.4) },
			{ recipient: royaltyRecipient, amount: etherToWei(0.1) },
		],
	}

	return {
		client: client as unknown as MemoryClient,
		contract,
		source: readFileSync(path, 'utf8'),
		order,
		offerer,
		fulfiller,
		royaltyRecipient,
	}
}
