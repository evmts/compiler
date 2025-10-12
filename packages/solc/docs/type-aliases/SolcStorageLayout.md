[**@tevm/solc**](../README.md)

***

[@tevm/solc](../globals.md) / SolcStorageLayout

# Type Alias: SolcStorageLayout\<T\>

> **SolcStorageLayout**\<`T`\> = `object`

Defined in: [solcTypes.ts:450](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L450)

The storage layout for a contract.

## Type Parameters

### T

`T` *extends* [`SolcStorageLayoutTypes`](SolcStorageLayoutTypes.md) = [`SolcStorageLayoutTypes`](SolcStorageLayoutTypes.md)

## Properties

### storage

> **storage**: [`SolcStorageLayoutItem`](SolcStorageLayoutItem.md)\<`T`\>[]

Defined in: [solcTypes.ts:455](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L455)

The list of stored variables with relevant slot information, type and metadata.

#### See

[SolcStorageLayoutItem](SolcStorageLayoutItem.md)

***

### types

> **types**: `T`

Defined in: [solcTypes.ts:460](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L460)

A record of all types relevant to the stored variables with additional encoding information.

#### See

[SolcStorageLayoutTypes](SolcStorageLayoutTypes.md)
