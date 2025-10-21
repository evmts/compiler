import type {
  Ast,
  CompileOutputJson,
  CompilerError,
  ContractBytecode,
  ContractState,
  SourceArtifactsJson,
} from "../../build";

type WithPathKey<TPath, TValue> = TValue extends SourceArtifacts<infer _>
  ? SourceArtifacts<Extract<TPath, string>>
  : TValue;

type ReadonlyRecord<K extends PropertyKey, V> = Readonly<{
  [P in K]: WithPathKey<P, V>;
}>;

type ReadonlyPartialRecord<K extends PropertyKey, V> = Readonly<
  Partial<{ [P in K]: WithPathKey<P, V> }>
>;

type ArtifactMap<
  THasErrors extends boolean,
  TPaths extends readonly string[] | undefined
> = TPaths extends readonly string[]
  ? THasErrors extends false
    ? ReadonlyRecord<TPaths[number], SourceArtifacts>
    : ReadonlyPartialRecord<TPaths[number], SourceArtifacts>
  : never;

type ArtifactValue<
  THasErrors extends boolean,
  TPaths extends readonly string[] | undefined
> = TPaths extends undefined
  ? THasErrors extends false
    ? SourceArtifacts
    : SourceArtifacts | undefined
  : never;

export declare class CompileOutput<
  THasErrors extends boolean = boolean,
  TSourcePaths extends readonly string[] | undefined = string[] | undefined
> {
  constructor();
  get artifactsJson(): Record<string, unknown>;
  get artifacts(): ArtifactMap<THasErrors, TSourcePaths>;
  get artifact(): ArtifactValue<THasErrors, TSourcePaths>;
  get errors(): THasErrors extends true
    ? ReadonlyArray<CompilerError>
    : undefined;
  get diagnostics(): Array<CompilerError>;
  hasCompilerErrors(): this is CompileOutput<true, TSourcePaths>;
  toJson(): CompileOutputJson;
}

type ContractStateInput = { name: string } & Partial<
  Omit<ContractState, "name">
>;

type ContractStateKeys = keyof ContractState;
type MutableContractStateKeys = Exclude<ContractStateKeys, "name">;

type NormalizeValue<Value> = [Exclude<Value, undefined>] extends [never]
  ? undefined
  : Exclude<Value, undefined>;

type DefinedValue<Key extends MutableContractStateKeys> = NormalizeValue<
  ContractState[Key]
>;

export type ContractStateMap = Partial<{
  [Key in MutableContractStateKeys]: DefinedValue<Key>;
}>;

export type ContractSnapshot<
  Name extends string,
  Map extends ContractStateMap
> = {
  name: Name;
} & {
  [Key in MutableContractStateKeys]: Map extends { [P in Key]-?: infer Value }
    ? Value
    : undefined;
};

export type FieldValue<
  Map extends ContractStateMap,
  Key extends MutableContractStateKeys
> = Map extends { [P in Key]-?: infer Value }
  ? [Value] extends [never]
    ? undefined
    : Value
  : undefined;

type ExtractDefinedValue<
  Input extends ContractStateInput,
  Key extends MutableContractStateKeys
> = NormalizeValue<
  Input extends { [P in Key]-?: infer Value } ? Value : undefined
>;

type StateMapFromInput<Input extends ContractStateInput> = {
  [Key in MutableContractStateKeys as ExtractDefinedValue<
    Input,
    Key
  > extends never
    ? never
    : Key]: ExtractDefinedValue<Input, Key>;
};

type UpdateMap<
  Map extends ContractStateMap,
  Key extends MutableContractStateKeys,
  Value
> = [NormalizeValue<Value>] extends [never]
  ? Omit<Map, Key>
  : Omit<Map, Key> & { [P in Key]: NormalizeValue<Value> };

type BytecodeMapValue<Next> = Next extends undefined
  ? undefined
  : Next extends null
  ? null
  : ContractBytecode;

type DefaultContractState = ContractSnapshot<string, {}>;

type FullyDefinedMap = {
  [Key in MutableContractStateKeys]: DefinedValue<Key>;
};

type NameOf<State extends ContractSnapshot<string, ContractStateMap>> =
  State extends ContractSnapshot<infer Name, any> ? Name : string;

type MapOf<State extends ContractSnapshot<string, ContractStateMap>> =
  State extends ContractSnapshot<string, infer Map> ? Map : {};

type ContractStateSnapshot<State extends ContractStateInput> = ContractSnapshot<
  State["name"],
  StateMapFromInput<State>
>;

type ContractStateShape = ContractSnapshot<string, ContractStateMap>;

type SnapshotToInput<State extends ContractStateShape> = {
  name: NameOf<State>;
} & {
  [Key in MutableContractStateKeys]?: MapOf<State> extends {
    [P in Key]-?: infer Value;
  }
    ? Value | undefined
    : undefined;
};

type ContractBytecodeStateValue<
  Next extends Uint8Array | `0x${string}` | string | null | undefined
> = BytecodeMapValue<Next>;

type UpdateField<
  State extends ContractStateShape,
  Key extends MutableContractStateKeys,
  Value
> = ContractSnapshot<NameOf<State>, UpdateMap<MapOf<State>, Key, Value>>;

type ContractStateAllDefined = ContractSnapshot<string, FullyDefinedMap>;

export interface Contract<
  Name extends string = string,
  Map extends ContractStateMap = {}
> {
  readonly __state: ContractSnapshot<Name, Map>;
  readonly name: Name;
  readonly address: FieldValue<Map, "address">;
  readonly creationBytecode: FieldValue<Map, "creationBytecode">;
  readonly deployedBytecode: FieldValue<Map, "deployedBytecode">;
  readonly abi: FieldValue<Map, "abi">;
  readonly metadata: FieldValue<Map, "metadata">;
  readonly userdoc: FieldValue<Map, "userdoc">;
  readonly devdoc: FieldValue<Map, "devdoc">;
  readonly storageLayout: FieldValue<Map, "storageLayout">;
  readonly immutableReferences: FieldValue<Map, "immutableReferences">;
  readonly methodIdentifiers: FieldValue<Map, "methodIdentifiers">;
  readonly functionDebugData: FieldValue<Map, "functionDebugData">;
  readonly gasEstimates: FieldValue<Map, "gasEstimates">;
  readonly assembly: FieldValue<Map, "assembly">;
  readonly legacyAssembly: FieldValue<Map, "legacyAssembly">;
  readonly opcodes: FieldValue<Map, "opcodes">;
  readonly ir: FieldValue<Map, "ir">;
  readonly irOptimized: FieldValue<Map, "irOptimized">;
  readonly ewasm: FieldValue<Map, "ewasm">;
  readonly creationSourceMap: FieldValue<Map, "creationSourceMap">;
  withAddress<
    NextAddress extends `0x${string}` | null | undefined =
      | `0x${string}`
      | null
      | undefined
  >(
    address?: NextAddress
  ): Contract<Name, UpdateMap<Map, "address", NextAddress>>;
  withCreationBytecode(): Contract<
    Name,
    UpdateMap<Map, "creationBytecode", undefined>
  >;
  withCreationBytecode(
    bytecode: null
  ): Contract<Name, UpdateMap<Map, "creationBytecode", null>>;
  withCreationBytecode(
    bytecode: Uint8Array | `0x${string}` | string
  ): Contract<Name, UpdateMap<Map, "creationBytecode", ContractBytecode>>;
  withCreationBytecode(
    bytecode?: Uint8Array | `0x${string}` | string | null
  ): Contract<
    Name,
    UpdateMap<Map, "creationBytecode", BytecodeMapValue<typeof bytecode>>
  >;
  withDeployedBytecode(): Contract<
    Name,
    UpdateMap<Map, "deployedBytecode", undefined>
  >;
  withDeployedBytecode(
    bytecode: null
  ): Contract<Name, UpdateMap<Map, "deployedBytecode", null>>;
  withDeployedBytecode(
    bytecode: Uint8Array | `0x${string}` | string
  ): Contract<Name, UpdateMap<Map, "deployedBytecode", ContractBytecode>>;
  withDeployedBytecode(
    bytecode?: Uint8Array | `0x${string}` | string | null
  ): Contract<
    Name,
    UpdateMap<Map, "deployedBytecode", BytecodeMapValue<typeof bytecode>>
  >;
  toJson(): ContractSnapshot<Name, Map>;
}

export interface ContractConstructor {
  new <StateInput extends ContractStateInput>(state: StateInput): Contract<
    StateInput["name"],
    StateMapFromInput<StateInput>
  >;
  fromSolcContractOutput(
    name: string,
    contract: object | string
  ): Contract<string, FullyDefinedMap>;
  readonly prototype: Contract;
}

export declare const Contract: ContractConstructor;

export declare class SourceArtifacts<TPath extends string = string> {
  constructor();
  get sourcePath(): TPath | null;
  get sourceId(): number | null;
  get solcVersion(): string | null;
  get ast(): Ast | undefined;
  get contracts(): Record<string, Contract>;
  toJson(): SourceArtifactsJson;
}
