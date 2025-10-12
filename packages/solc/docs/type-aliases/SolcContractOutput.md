[**@tevm/solc**](../README.md)

***

[@tevm/solc](../globals.md) / SolcContractOutput

# Type Alias: SolcContractOutput

> **SolcContractOutput** = `object`

Defined in: [solcTypes.ts:416](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L416)

## Properties

### abi

> **abi**: `Abi`

Defined in: [solcTypes.ts:418](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L418)

***

### devdoc

> **devdoc**: `any`

Defined in: [solcTypes.ts:432](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L432)

***

### evm

> **evm**: [`SolcEvmOutput`](SolcEvmOutput.md)

Defined in: [solcTypes.ts:441](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L441)

***

### ewasm

> **ewasm**: [`SolcEwasmOutput`](SolcEwasmOutput.md)

Defined in: [solcTypes.ts:444](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L444)

***

### ir

> **ir**: `string`

Defined in: [solcTypes.ts:435](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L435)

***

### metadata

> **metadata**: `string`

Defined in: [solcTypes.ts:421](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L421)

***

### storageLayout

> **storageLayout**: [`SolcStorageLayout`](SolcStorageLayout.md)

Defined in: [solcTypes.ts:438](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L438)

***

### userdoc

> **userdoc**: `object`

Defined in: [solcTypes.ts:424](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L424)

#### kind

> **kind**: `"user"`

#### methods?

> `optional` **methods**: `Record`\<`string`, \{ `notice`: `string`; \}\>

#### notice?

> `optional` **notice**: `string`

#### version

> **version**: `number`
