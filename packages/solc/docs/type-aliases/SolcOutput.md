[**@tevm/solc**](../README.md)

***

[@tevm/solc](../globals.md) / SolcOutput

# Type Alias: SolcOutput\<T\>

> **SolcOutput**\<`T`\> = `object`

Defined in: [solcTypes.ts:353](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L353)

## Type Parameters

### T

`T` *extends* [`SolcLanguage`](SolcLanguage.md) = [`SolcLanguage`](SolcLanguage.md)

## Properties

### contracts?

> `optional` **contracts**: `object`

Defined in: [solcTypes.ts:365](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L365)

#### Index Signature

\[`sourceFile`: `string`\]: `object`

***

### errors?

> `optional` **errors**: [`SolcErrorEntry`](SolcErrorEntry.md)[]

Defined in: [solcTypes.ts:355](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L355)

***

### sources

> **sources**: `object`

Defined in: [solcTypes.ts:359](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L359)

#### Index Signature

\[`sourceFile`: `string`\]: [`SolcSourceEntry`](SolcSourceEntry.md)
