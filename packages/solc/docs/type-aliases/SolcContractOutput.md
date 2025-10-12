[**@tevm/solc**](../README.md)

***

[@tevm/solc](../globals.md) / SolcContractOutput

# Type Alias: SolcContractOutput

> **SolcContractOutput** = `object`

Defined in: [solcTypes.ts:417](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L417)

## Properties

### abi

> **abi**: `Abi`

Defined in: [solcTypes.ts:419](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L419)

***

### devdoc

> **devdoc**: `any`

Defined in: [solcTypes.ts:433](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L433)

***

### evm

> **evm**: [`SolcEvmOutput`](SolcEvmOutput.md)

Defined in: [solcTypes.ts:442](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L442)

***

### ewasm

> **ewasm**: [`SolcEwasmOutput`](SolcEwasmOutput.md)

Defined in: [solcTypes.ts:445](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L445)

***

### ir

> **ir**: `string`

Defined in: [solcTypes.ts:436](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L436)

***

### metadata

> **metadata**: `string`

Defined in: [solcTypes.ts:422](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L422)

***

### storageLayout

> **storageLayout**: [`SolcStorageLayout`](SolcStorageLayout.md)

Defined in: [solcTypes.ts:439](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L439)

***

### userdoc

> **userdoc**: `object`

Defined in: [solcTypes.ts:425](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L425)

#### kind

> **kind**: `"user"`

#### methods?

> `optional` **methods**: `Record`\<`string`, \{ `notice`: `string`; \}\>

#### notice?

> `optional` **notice**: `string`

#### version

> **version**: `number`
