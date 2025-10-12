[**@tevm/solc**](../README.md)

***

[@tevm/solc](../globals.md) / SolcStorageLayoutStructType

# Interface: SolcStorageLayoutStructType

Defined in: [solcTypes.ts:574](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L574)

A storage layout type that is a struct.

## Extends

- [`SolcStorageLayoutInplaceType`](SolcStorageLayoutInplaceType.md)

## Properties

### encoding

> **encoding**: `"inplace"`

Defined in: [solcTypes.ts:535](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L535)

How the data is encoded in storage

- inplace: data is laid out contiguously in storage
- mapping: keccak-256 hash-based method
- dynamic_array: keccak-256 hash-based method
- bytes: single slot or keccak-256 hash-based depending on the data size

#### Inherited from

[`SolcStorageLayoutInplaceType`](SolcStorageLayoutInplaceType.md).[`encoding`](SolcStorageLayoutInplaceType.md#encoding)

***

### label

> **label**: `string`

Defined in: [solcTypes.ts:522](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L522)

The canonical type name

#### Inherited from

[`SolcStorageLayoutInplaceType`](SolcStorageLayoutInplaceType.md).[`label`](SolcStorageLayoutInplaceType.md#label)

***

### members

> **members**: [`SolcStorageLayoutItem`](../type-aliases/SolcStorageLayoutItem.md)\<[`SolcStorageLayoutTypes`](../type-aliases/SolcStorageLayoutTypes.md)\>[]

Defined in: [solcTypes.ts:578](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L578)

The members of the struct in the same format as a [SolcStorageLayoutItem](../type-aliases/SolcStorageLayoutItem.md)

***

### numberOfBytes

> **numberOfBytes**: `string`

Defined in: [solcTypes.ts:528](https://github.com/evmts/compiler/blob/main/packages/solc/src/solcTypes.ts#L528)

The number of used bytes (as a decimal string)

Note: if numberOfBytes > 32 this means that more than one slot is used

#### Inherited from

[`SolcStorageLayoutInplaceType`](SolcStorageLayoutInplaceType.md).[`numberOfBytes`](SolcStorageLayoutInplaceType.md#numberofbytes)
