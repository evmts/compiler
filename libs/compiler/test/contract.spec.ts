import { describe, expect, test } from 'bun:test'
import { Contract } from '../build/index.js'

const sampleSolcContract = {
	abi: [],
	userdoc: {},
	devdoc: {},
	storageLayout: { storage: [] },
	evm: {
		bytecode: {
			object: '0x6000',
			linkReferences: {},
			sourceMap: '00',
		},
		deployedBytecode: {
			bytecode: { object: '0x6001', linkReferences: {} },
			immutableReferences: {},
		},
	},
}

const sampleConfigurableArtifact = {
	abi: [],
	bytecode: {
		object: '0x6002',
		linkReferences: {},
		sourceMap: '11',
	},
	deployedBytecode: {
		bytecode: { object: '0x6003', linkReferences: {} },
		immutableReferences: {},
	},
	storageLayout: { storage: [] },
	userdoc: { notice: 'notice' },
	devdoc: { details: 'details' },
	rawMetadata: '{"compiler":"solc"}',
	methodIdentifiers: { 'foo()': '0xdeadbeef' },
}

describe('Contract', () => {
	test('creates default state from name', () => {
		const contract = new Contract({ name: 'Manual' })
		const snapshot = contract.toJson()
		expect(snapshot.name).toBe('Manual')
		expect(snapshot.address).toBeUndefined()
		expect(snapshot.creationBytecode).toBeUndefined()
	})

	test('fromSolcOutput hydrates compiler metadata', () => {
		const contract = Contract.fromSolcContractOutput('FromSolc', sampleSolcContract)
		const snapshot = contract.toJson()
		expect(snapshot.name).toBe('FromSolc')
		expect(snapshot.creationSourceMap).toBe('00')
		expect(snapshot.methodIdentifiers).toBeUndefined()
		expect(snapshot.userdoc).toEqual({})
		expect(snapshot.devdoc).toEqual({})
	})

	test('fromConfigurableArtifact absorbs optional sections', () => {
		const solcLike = {
			abi: sampleConfigurableArtifact.abi,
			userdoc: sampleConfigurableArtifact.userdoc,
			devdoc: sampleConfigurableArtifact.devdoc,
			storageLayout: sampleConfigurableArtifact.storageLayout,
			evm: {
				bytecode: {
					object: sampleConfigurableArtifact.bytecode.object,
					sourceMap: sampleConfigurableArtifact.bytecode.sourceMap,
				},
				deployedBytecode: {
					bytecode: {
						object: sampleConfigurableArtifact.deployedBytecode.bytecode.object,
					},
					immutableReferences: sampleConfigurableArtifact.deployedBytecode.immutableReferences,
				},
				methodIdentifiers: sampleConfigurableArtifact.methodIdentifiers,
			},
		}

		const contract = Contract.fromSolcContractOutput('FromConfig', solcLike)
		const snapshot = contract.toJson()
		expect(snapshot.name).toBe('FromConfig')
		expect(snapshot.userdoc).toEqual({ notice: 'notice' })
		expect(snapshot.devdoc).toEqual({ details: 'details' })
		expect(snapshot.methodIdentifiers).toEqual({
			'foo()': '0xdeadbeef',
		})
	})

	test('mutator chaining updates tweakable fields', () => {
		const contract = new Contract({ name: 'Tweaks' })
			.withAddress('0x1234')
			.withCreationBytecode(Buffer.from([0xde, 0xad]))
			.withDeployedBytecode(Buffer.from([0xca, 0xfe]))

		const snapshot = contract.toJson()
		expect(snapshot.address).toBe('0x1234')
		expect(snapshot.creationBytecode?.hex).toBe('0xdead')
		expect(snapshot.deployedBytecode?.hex).toBe('0xcafe')
	})

	test('hex string bytecode is accepted', () => {
		const creationHex = '0xDEADBEEF'
		const deployedHex = '0xFEEDFACE'

		const contract = new Contract({ name: 'Hex' }).withCreationBytecode(creationHex).withDeployedBytecode(deployedHex)

		const snapshot = contract.toJson()
		expect(snapshot.creationBytecode?.hex).toBe('0xdeadbeef')
		expect(Array.from(snapshot.creationBytecode?.bytes ?? [])).toEqual([0xde, 0xad, 0xbe, 0xef])
		expect(snapshot.deployedBytecode?.hex).toBe('0xfeedface')
		expect(Array.from(snapshot.deployedBytecode?.bytes ?? [])).toEqual([0xfe, 0xed, 0xfa, 0xce])
	})

	test('constructor accepts existing state', () => {
		const base = Contract.fromSolcContractOutput('Existing', sampleSolcContract)
		const snapshot = base.toJson()

		const cloned = new Contract(snapshot)
		expect(cloned.toJson()).toMatchObject(snapshot)
	})
})
