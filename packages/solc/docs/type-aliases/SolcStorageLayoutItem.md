[**@tevm/solc**](../README.md)

***

[@tevm/solc](../globals.md) / SolcStorageLayoutItem

# Type Alias: SolcStorageLayoutItem\<T\>

> **SolcStorageLayoutItem**\<`T`\> = `object`

Defined in: [solcTypes.ts:468](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L468)

An item present in the contract's storage

## See

[Solidity documentation](https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html#json-output)

## Type Parameters

### T

`T` *extends* [`SolcStorageLayoutTypes`](SolcStorageLayoutTypes.md) = [`SolcStorageLayoutTypes`](SolcStorageLayoutTypes.md)

## Properties

### astId

> **astId**: `number`

Defined in: [solcTypes.ts:472](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L472)

The id of the AST node of the state variable's declaration

***

### contract

> **contract**: `string`

Defined in: [solcTypes.ts:476](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L476)

The name of the contract including its path as prefix

***

### label

> **label**: `string`

Defined in: [solcTypes.ts:480](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L480)

The name of the state variable

***

### offset

> **offset**: `number`

Defined in: [solcTypes.ts:484](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L484)

The offset in bytes within the storage slot according to the encoding

***

### slot

> **slot**: `string`

Defined in: [solcTypes.ts:488](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L488)

The storage slot where the state variable resides or starts

***

### type

> **type**: keyof `T`

Defined in: [solcTypes.ts:492](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L492)

The identifier used as a key to the variable's type information in the [SolcStorageLayoutTypes](SolcStorageLayoutTypes.md) record
