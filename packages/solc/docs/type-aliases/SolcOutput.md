[**@tevm/solc**](../README.md)

***

[@tevm/solc](../globals.md) / SolcOutput

# Type Alias: SolcOutput\<T\>

> **SolcOutput**\<`T`\> = `object`

Defined in: [solcTypes.ts:352](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L352)

## Type Parameters

### T

`T` *extends* [`SolcLanguage`](SolcLanguage.md) = [`SolcLanguage`](SolcLanguage.md)

## Properties

### contracts?

> `optional` **contracts**: `object`

Defined in: [solcTypes.ts:364](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L364)

#### Index Signature

\[`sourceFile`: `string`\]: `object`

***

### errors?

> `optional` **errors**: [`SolcErrorEntry`](SolcErrorEntry.md)[]

Defined in: [solcTypes.ts:354](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L354)

***

### sources

> **sources**: `object`

Defined in: [solcTypes.ts:358](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L358)

#### Index Signature

\[`sourceFile`: `string`\]: [`SolcSourceEntry`](SolcSourceEntry.md)
