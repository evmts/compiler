[**@tevm/solc**](../README.md)

***

[@tevm/solc](../globals.md) / Solc

# Interface: Solc

Defined in: solcTypes.ts:885

## Properties

### compile()

> **compile**: (`input`) => [`SolcOutput`](../type-aliases/SolcOutput.md)

Defined in: solcTypes.ts:891

#### Parameters

##### input

[`SolcInputDescription`](../type-aliases/SolcInputDescription.md)

#### Returns

[`SolcOutput`](../type-aliases/SolcOutput.md)

***

### features

> **features**: `FeaturesConfig`

Defined in: solcTypes.ts:890

***

### license

> **license**: `string`

Defined in: solcTypes.ts:888

***

### loadRemoteVersion()

> **loadRemoteVersion**: (`versionString`, `callback`) => `void`

Defined in: solcTypes.ts:892

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

Defined in: solcTypes.ts:889

***

### semver

> **semver**: `string`

Defined in: solcTypes.ts:887

***

### setupMethods()

> **setupMethods**: (`soljson`) => `void`

Defined in: solcTypes.ts:893

#### Parameters

##### soljson

`any`

#### Returns

`void`

***

### version

> **version**: `string`

Defined in: solcTypes.ts:886
