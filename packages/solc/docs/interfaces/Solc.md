[**@tevm/solc**](../README.md)

***

[@tevm/solc](../globals.md) / Solc

# Interface: Solc

Defined in: [solcTypes.ts:909](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L909)

## Properties

### compile()

> **compile**: \<`T`\>(`input`) => [`SolcOutput`](../type-aliases/SolcOutput.md)\<`T`\>

Defined in: [solcTypes.ts:915](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L915)

#### Type Parameters

##### T

`T` *extends* [`SolcLanguage`](../type-aliases/SolcLanguage.md) = [`SolcLanguage`](../type-aliases/SolcLanguage.md)

#### Parameters

##### input

[`SolcInputDescription`](../type-aliases/SolcInputDescription.md)\<`T`\>

#### Returns

[`SolcOutput`](../type-aliases/SolcOutput.md)\<`T`\>

***

### features

> **features**: `FeaturesConfig`

Defined in: [solcTypes.ts:914](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L914)

***

### license

> **license**: `string`

Defined in: [solcTypes.ts:912](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L912)

***

### loadRemoteVersion()

> **loadRemoteVersion**: (`versionString`, `callback`) => `void`

Defined in: [solcTypes.ts:916](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L916)

#### Parameters

##### versionString

`string`

##### callback

(`err`, `solc?`) => `void`

#### Returns

`void`

***

### lowlevel

> **lowlevel**: `LowLevelConfig`

Defined in: [solcTypes.ts:913](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L913)

***

### semver

> **semver**: `string`

Defined in: [solcTypes.ts:911](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L911)

***

### setupMethods()

> **setupMethods**: (`soljson`) => `void`

Defined in: [solcTypes.ts:917](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L917)

#### Parameters

##### soljson

`any`

#### Returns

`void`

***

### version

> **version**: `string`

Defined in: [solcTypes.ts:910](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L910)
