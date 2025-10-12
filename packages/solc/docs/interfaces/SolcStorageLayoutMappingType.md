[**@tevm/solc**](../README.md)

***

[@tevm/solc](../globals.md) / SolcStorageLayoutMappingType

# Interface: SolcStorageLayoutMappingType

Defined in: [solcTypes.ts:548](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L548)

A storage layout type that is laid out in a keccak-256 hash-based method.

## Extends

- [`SolcStorageLayoutTypeBase`](SolcStorageLayoutTypeBase.md)

## Properties

### encoding

> **encoding**: `"mapping"`

Defined in: [solcTypes.ts:549](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L549)

How the data is encoded in storage

- inplace: data is laid out contiguously in storage
- mapping: keccak-256 hash-based method
- dynamic_array: keccak-256 hash-based method
- bytes: single slot or keccak-256 hash-based depending on the data size

#### Overrides

[`SolcStorageLayoutTypeBase`](SolcStorageLayoutTypeBase.md).[`encoding`](SolcStorageLayoutTypeBase.md#encoding)

***

### key

> **key**: `` `t_${string}` ``

Defined in: [solcTypes.ts:553](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L553)

The associated type for the mapping key

***

### label

> **label**: `string`

Defined in: [solcTypes.ts:522](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L522)

The canonical type name

#### Inherited from

[`SolcStorageLayoutTypeBase`](SolcStorageLayoutTypeBase.md).[`label`](SolcStorageLayoutTypeBase.md#label)

***

### numberOfBytes

> **numberOfBytes**: `string`

Defined in: [solcTypes.ts:528](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L528)

The number of used bytes (as a decimal string)

Note: if numberOfBytes > 32 this means that more than one slot is used

#### Inherited from

[`SolcStorageLayoutTypeBase`](SolcStorageLayoutTypeBase.md).[`numberOfBytes`](SolcStorageLayoutTypeBase.md#numberofbytes)

***

### value

> **value**: `` `t_${string}` ``

Defined in: [solcTypes.ts:557](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L557)

The associated type for the mapping value
