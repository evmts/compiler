/**
 * The storage layout for a contract.
 */
export type SolcStorageLayout<T extends SolcStorageLayoutTypes = SolcStorageLayoutTypes> = {
	/**
	 * The list of stored variables with relevant slot information, type and metadata.
	 * @see {@link SolcStorageLayoutItem}
	 */
	storage: Array<SolcStorageLayoutItem<T>>
	/**
	 * A record of all types relevant to the stored variables with additional encoding information.
	 * @see {@link SolcStorageLayoutTypes}
	 */
	types: T
}

export type StorageLayout<T extends SolcStorageLayoutTypes = SolcStorageLayoutTypes> =
	SolcStorageLayout<T>;

/**
 * An item present in the contract's storage
 * @see [Solidity documentation](https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html#json-output)
 */
export type SolcStorageLayoutItem<T extends SolcStorageLayoutTypes = SolcStorageLayoutTypes> = {
	/**
	 * The id of the AST node of the state variable's declaration
	 */
	astId: number
	/**
	 * The name of the contract including its path as prefix
	 */
	contract: string
	/**
	 * The name of the state variable
	 */
	label: string
	/**
	 * The offset in bytes within the storage slot according to the encoding
	 */
	offset: number
	/**
	 * The storage slot where the state variable resides or starts
	 */
	slot: string
	/**
	 * The identifier used as a key to the variable's type information in the {@link SolcStorageLayoutTypes} record
	 */
	type: keyof T
}

/**
 * A record of all types relevant to the stored variables mapped to their encoding information.
 */
export type SolcStorageLayoutTypes = Record<
	`t_${string}`,
	| SolcStorageLayoutInplaceType
	| SolcStorageLayoutBytesType
	| SolcStorageLayoutMappingType
	| SolcStorageLayoutDynamicArrayType
	| SolcStorageLayoutStructType
>

/**
 * The base type for all storage layout types.
 */
export interface SolcStorageLayoutTypeBase {
	/**
	 * How the data is encoded in storage
	 *
	 * - inplace: data is laid out contiguously in storage
	 * - mapping: keccak-256 hash-based method
	 * - dynamic_array: keccak-256 hash-based method
	 * - bytes: single slot or keccak-256 hash-based depending on the data size
	 */
	encoding: 'inplace' | 'mapping' | 'dynamic_array' | 'bytes'
	/**
	 * The canonical type name
	 */
	label: string
	/**
	 * The number of used bytes (as a decimal string)
	 *
	 * Note: if numberOfBytes > 32 this means that more than one slot is used
	 */
	numberOfBytes: string
}

/**
 * A storage layout type that is laid out contiguously in storage.
 */
export interface SolcStorageLayoutInplaceType extends SolcStorageLayoutTypeBase {
	encoding: 'inplace'
}

/**
 * A storage layout type that is laid out in a single slot or keccak-256 hash-based depending on the data size.
 */
export interface SolcStorageLayoutBytesType extends SolcStorageLayoutTypeBase {
	encoding: 'bytes'
}

/**
 * A storage layout type that is laid out in a keccak-256 hash-based method.
 */
export interface SolcStorageLayoutMappingType extends SolcStorageLayoutTypeBase {
	encoding: 'mapping'
	/**
	 * The associated type for the mapping key
	 */
	key: `t_${string}`
	/**
	 * The associated type for the mapping value
	 */
	value: `t_${string}`
}

/**
 * A storage layout type that is laid out in a keccak-256 hash-based method.
 */
export interface SolcStorageLayoutDynamicArrayType extends SolcStorageLayoutTypeBase {
	encoding: 'dynamic_array'
	/**
	 * The base type of the dynamic array
	 */
	base: `t_${string}`
}

/**
 * A storage layout type that is a struct.
 */
export interface SolcStorageLayoutStructType extends SolcStorageLayoutInplaceType {
	/**
	 * The members of the struct in the same format as a {@link SolcStorageLayoutItem}
	 */
	members: Array<SolcStorageLayoutItem>
}
