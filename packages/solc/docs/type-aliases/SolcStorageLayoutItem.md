[**@tevm/solc**](../README.md)

***

[@tevm/solc](../globals.md) / SolcStorageLayoutItem

# Type Alias: SolcStorageLayoutItem\<T\>

> **SolcStorageLayoutItem**\<`T`\> = `object`

Defined in: solcTypes.ts:444

An item present in the contract's storage

## See

[Solidity documentation](https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html#json-output)

## Type Parameters

### T

`T` *extends* [`SolcStorageLayoutTypes`](SolcStorageLayoutTypes.md) = [`SolcStorageLayoutTypes`](SolcStorageLayoutTypes.md)

## Properties

### astId

> **astId**: `number`

Defined in: solcTypes.ts:448

The id of the AST node of the state variable's declaration

***

### contract

> **contract**: `string`

Defined in: solcTypes.ts:452

The name of the contract including its path as prefix

***

### label

> **label**: `string`

Defined in: solcTypes.ts:456

The name of the state variable

***

### offset

> **offset**: `number`

Defined in: solcTypes.ts:460

The offset in bytes within the storage slot according to the encoding

***

### slot

> **slot**: `string`

Defined in: solcTypes.ts:464

The storage slot where the state variable resides or starts

***

### type

> **type**: keyof `T`

Defined in: solcTypes.ts:468

The identifier used as a key to the variable's type information in the [SolcStorageLayoutTypes](SolcStorageLayoutTypes.md) record
