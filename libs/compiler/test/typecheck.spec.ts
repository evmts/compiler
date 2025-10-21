import {
  Contract,
  type CompileOutput,
  type CompilerError,
  type ContractBytecode,
  type SourceArtifacts,
} from "../build/index.js";

type Expect<T extends true> = T;
type Equal<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;
type ContractInstanceState<T> = T extends { readonly __state?: infer State }
  ? State
  : never;
type NoUndefined<T> = Extract<T, undefined> extends never ? true : false;

type SinglePath = readonly ["contracts/Only.sol"];
type MultiPath = readonly ["contracts/A.sol", "contracts/B.sol"];
type SourcePaths = readonly ["Foo.sol", "Bar.sol"];

type SingleFilesUnion =
  | CompileOutput<false, SinglePath>
  | CompileOutput<true, SinglePath>;
type SingleFilesSuccess = Extract<
  SingleFilesUnion,
  CompileOutput<false, SinglePath>
>;
type SingleFilesFailure = Extract<
  SingleFilesUnion,
  CompileOutput<true, SinglePath>
>;

type _SingleFilesSuccessArtifactsAssignable = Expect<
  Equal<
    SingleFilesSuccess["artifacts"] extends Readonly<
      Record<SinglePath[number], SourceArtifacts<SinglePath[number]>>
    >
      ? true
      : false,
    true
  >
>;
type _SingleFilesSuccessArtifactsSuper = Expect<
  Equal<
    Readonly<
      Record<SinglePath[number], SourceArtifacts<SinglePath[number]>>
    > extends SingleFilesSuccess["artifacts"]
      ? true
      : false,
    true
  >
>;
type _SingleFilesFailureArtifactsAssignable = Expect<
  Equal<
    SingleFilesFailure["artifacts"] extends Readonly<
      Partial<Record<SinglePath[number], SourceArtifacts<SinglePath[number]>>>
    >
      ? true
      : false,
    true
  >
>;
type _SingleFilesFailureArtifactsSuper = Expect<
  Equal<
    Readonly<
      Partial<Record<SinglePath[number], SourceArtifacts<SinglePath[number]>>>
    > extends SingleFilesFailure["artifacts"]
      ? true
      : false,
    true
  >
>;
type _SingleFilesSuccessArtifact = Expect<
  Equal<SingleFilesSuccess["artifact"], never>
>;
type _SingleFilesFailureArtifact = Expect<
  Equal<SingleFilesFailure["artifact"], never>
>;
type _SingleFilesSuccessErrors = Expect<
  Equal<SingleFilesSuccess["errors"], undefined>
>;
type _SingleFilesFailureErrors = Expect<
  Equal<SingleFilesFailure["errors"], ReadonlyArray<CompilerError>>
>;

type MultiFilesUnion =
  | CompileOutput<false, MultiPath>
  | CompileOutput<true, MultiPath>;
type MultiFilesSuccess = Extract<
  MultiFilesUnion,
  CompileOutput<false, MultiPath>
>;
type MultiFilesFailure = Extract<
  MultiFilesUnion,
  CompileOutput<true, MultiPath>
>;

type _MultiFilesSuccessArtifactsAssignable = Expect<
  Equal<
    MultiFilesSuccess["artifacts"] extends Readonly<
      Record<MultiPath[number], SourceArtifacts<MultiPath[number]>>
    >
      ? true
      : false,
    true
  >
>;
type _MultiFilesSuccessArtifactsSuper = Expect<
  Equal<
    Readonly<
      Readonly<{
        "contracts/A.sol": SourceArtifacts<"contracts/A.sol">;
        "contracts/B.sol": SourceArtifacts<"contracts/B.sol">;
      }>
    > extends MultiFilesSuccess["artifacts"]
      ? true
      : false,
    true
  >
>;
type _MultiFilesFailureArtifactsAssignable = Expect<
  Equal<
    MultiFilesFailure["artifacts"] extends Readonly<
      Partial<Record<MultiPath[number], SourceArtifacts<MultiPath[number]>>>
    >
      ? true
      : false,
    true
  >
>;
type _MultiFilesFailureArtifactsSuper = Expect<
  Equal<
    Readonly<
      Partial<{
        "contracts/A.sol": SourceArtifacts<"contracts/A.sol">;
        "contracts/B.sol": SourceArtifacts<"contracts/B.sol">;
      }>
    > extends MultiFilesFailure["artifacts"]
      ? true
      : false,
    true
  >
>;

type _MultiFilesSuccessErrors = Expect<
  Equal<MultiFilesSuccess["errors"], undefined>
>;
type _MultiFilesFailureErrors = Expect<
  Equal<MultiFilesFailure["errors"], ReadonlyArray<CompilerError>>
>;

type SourcesUnion =
  | CompileOutput<false, SourcePaths>
  | CompileOutput<true, SourcePaths>;
type SourcesSuccess = Extract<SourcesUnion, CompileOutput<false, SourcePaths>>;
type SourcesFailure = Extract<SourcesUnion, CompileOutput<true, SourcePaths>>;

type _SourcesSuccessArtifactsAssignable = Expect<
  Equal<
    SourcesSuccess["artifacts"] extends Readonly<
      Record<SourcePaths[number], SourceArtifacts<SourcePaths[number]>>
    >
      ? true
      : false,
    true
  >
>;
type _SourcesSuccessArtifactsSuper = Expect<
  Equal<
    Readonly<{
      "Foo.sol": SourceArtifacts<"Foo.sol">;
      "Bar.sol": SourceArtifacts<"Bar.sol">;
    }> extends SourcesSuccess["artifacts"]
      ? true
      : false,
    true
  >
>;
type _SourcesFailureArtifactsAssignable = Expect<
  Equal<
    SourcesFailure["artifacts"] extends Readonly<
      Partial<Record<SourcePaths[number], SourceArtifacts<SourcePaths[number]>>>
    >
      ? true
      : false,
    true
  >
>;
type _SourcesFailureArtifactsSuper = Expect<
  Equal<
    Readonly<
      Partial<{
        "Foo.sol": SourceArtifacts<"Foo.sol">;
        "Bar.sol": SourceArtifacts<"Bar.sol">;
      }>
    > extends SourcesFailure["artifacts"]
      ? true
      : false,
    true
  >
>;
type _SourcesSuccessErrors = Expect<Equal<SourcesSuccess["errors"], undefined>>;
type _SourcesFailureErrors = Expect<
  Equal<SourcesFailure["errors"], ReadonlyArray<CompilerError>>
>;

type SingleSourceUnion =
  | CompileOutput<false, undefined>
  | CompileOutput<true, undefined>;
type SingleSourceSuccess = Extract<
  SingleSourceUnion,
  CompileOutput<false, undefined>
>;
type SingleSourceFailure = Extract<
  SingleSourceUnion,
  CompileOutput<true, undefined>
>;

type _SingleSourceSuccessArtifact = Expect<
  Equal<SingleSourceSuccess["artifact"], SourceArtifacts>
>;
type _SingleSourceFailureArtifact = Expect<
  Equal<SingleSourceFailure["artifact"], SourceArtifacts | undefined>
>;
type _SingleSourceSuccessErrors = Expect<
  Equal<SingleSourceSuccess["errors"], undefined>
>;
type _SingleSourceFailureErrors = Expect<
  Equal<SingleSourceFailure["errors"], ReadonlyArray<CompilerError>>
>;

type SingleContractUnion =
  | CompileOutput<false, undefined>
  | CompileOutput<true, undefined>;
type SingleContractSuccess = Extract<
  SingleContractUnion,
  CompileOutput<false, undefined>
>;
type SingleContractFailure = Extract<
  SingleContractUnion,
  CompileOutput<true, undefined>
>;

type _SingleContractSuccessArtifact = Expect<
  Equal<SingleContractSuccess["artifact"], SourceArtifacts>
>;
type _SingleContractFailureArtifact = Expect<
  Equal<SingleContractFailure["artifact"], SourceArtifacts | undefined>
>;
type _SingleContractSuccessErrors = Expect<
  Equal<SingleContractSuccess["errors"], undefined>
>;
type _SingleContractFailureErrors = Expect<
  Equal<SingleContractFailure["errors"], ReadonlyArray<CompilerError>>
>;

type ProjectUnion =
  | CompileOutput<false, string[]>
  | CompileOutput<true, string[]>;
type ProjectSuccess = Extract<ProjectUnion, CompileOutput<false, string[]>>;
type ProjectFailure = Extract<ProjectUnion, CompileOutput<true, string[]>>;

type _ProjectSuccessArtifacts = Expect<
  Equal<ProjectSuccess["artifacts"], Readonly<Record<string, SourceArtifacts>>>
>;
type _ProjectFailureArtifacts = Expect<
  Equal<
    ProjectFailure["artifacts"],
    Readonly<Partial<Record<string, SourceArtifacts>>>
  >
>;
type _ProjectSuccessErrors = Expect<Equal<ProjectSuccess["errors"], undefined>>;
type _ProjectFailureErrors = Expect<
  Equal<ProjectFailure["errors"], ReadonlyArray<CompilerError>>
>;

type _TypeGuardAccessible = Expect<
  Equal<
    CompileOutput<boolean, SinglePath> extends {
      hasCompilerErrors(): this is CompileOutput<true, SinglePath>;
    }
      ? true
      : false,
    true
  >
>;

// -----------------------------------------------------------------------------
// Contract type inference
// -----------------------------------------------------------------------------

const bytecodeBytes = undefined as unknown as
  | Uint8Array
  | `0x${string}`
  | string;

const contractNameOnly = new Contract({ name: "ContractNameOnly" });
type ContractNameOnly = typeof contractNameOnly;
type ContractNameOnlyState = ContractInstanceState<ContractNameOnly>;
type _ContractNameOnlyAddressGetter = Expect<
  Equal<ContractNameOnly["address"], undefined>
>;
type _ContractNameOnlyStateAddress = Expect<
  Equal<ContractNameOnlyState["address"], undefined>
>;
type _ContractNameOnlyJsonMatchesState = Expect<
  Equal<ReturnType<ContractNameOnly["toJson"]>, ContractNameOnlyState>
>;

const contractWithAddressLiteral = new Contract({
  name: "ContractWithAddressLiteral",
  address: "0x1234" as `0x${string}`,
});
type ContractWithAddressLiteral = typeof contractWithAddressLiteral;
type ContractWithAddressLiteralState =
  ContractInstanceState<ContractWithAddressLiteral>;
type _ContractWithAddressLiteralGetter = Expect<
  Equal<ContractWithAddressLiteral["address"], `0x${string}`>
>;
type _ContractWithAddressLiteralState = Expect<
  Equal<ContractWithAddressLiteralState["address"], `0x${string}`>
>;

const contractWithNullAddress = new Contract({
  name: "ContractWithNullAddress",
  address: null,
});
type ContractWithNullAddress = typeof contractWithNullAddress;
type _ContractWithNullAddressGetter = Expect<
  Equal<ContractWithNullAddress["address"], null>
>;
type _ContractWithNullAddressState = Expect<
  Equal<ContractInstanceState<ContractWithNullAddress>["address"], null>
>;

const contractWithExplicitUndefinedAddress = new Contract({
  name: "ContractWithExplicitUndefinedAddress",
  address: undefined,
});
type _ContractWithExplicitUndefinedAddressGetter = Expect<
  Equal<(typeof contractWithExplicitUndefinedAddress)["address"], undefined>
>;

const contractWithAddressMutation = new Contract({
  name: "ContractWithAddressMutation",
}).withAddress("0xdeadbeef" as `0x${string}`);
type ContractWithAddressMutation = typeof contractWithAddressMutation;
type _ContractWithAddressMutationGetter = Expect<
  Equal<ContractWithAddressMutation["address"], `0x${string}`>
>;
type _ContractWithAddressMutationState = Expect<
  Equal<
    ContractInstanceState<ContractWithAddressMutation>["address"],
    `0x${string}`
  >
>;

const contractWithAddressCleared =
  contractWithAddressMutation.withAddress(undefined);
type _ContractWithAddressClearedGetter = Expect<
  Equal<(typeof contractWithAddressCleared)["address"], undefined>
>;

const contractWithAddressSetNull =
  contractWithAddressMutation.withAddress(null);
type _ContractWithAddressSetNullGetter = Expect<
  Equal<(typeof contractWithAddressSetNull)["address"], null>
>;

const contractWithCreationBytecode = new Contract({
  name: "ContractWithCreationBytecode",
}).withCreationBytecode(bytecodeBytes);
type _ContractWithCreationBytecodeGetter = Expect<
  Equal<
    (typeof contractWithCreationBytecode)["creationBytecode"],
    ContractBytecode
  >
>;
type _ContractWithCreationBytecodeState = Expect<
  Equal<
    ContractInstanceState<
      typeof contractWithCreationBytecode
    >["creationBytecode"],
    ContractBytecode
  >
>;

const contractWithCreationBytecodeNull = new Contract({
  name: "ContractWithCreationBytecodeNull",
}).withCreationBytecode(null);
type _ContractWithCreationBytecodeNullGetter = Expect<
  Equal<(typeof contractWithCreationBytecodeNull)["creationBytecode"], null>
>;

const contractWithCreationBytecodeHex = new Contract({
  name: "ContractWithCreationBytecodeHex",
}).withCreationBytecode("0xdeadbeef");
type _ContractWithCreationBytecodeHexGetter = Expect<
  Equal<
    (typeof contractWithCreationBytecodeHex)["creationBytecode"],
    ContractBytecode
  >
>;
type _ContractWithCreationBytecodeHexState = Expect<
  Equal<
    ContractInstanceState<
      typeof contractWithCreationBytecodeHex
    >["creationBytecode"],
    ContractBytecode
  >
>;

const contractWithDeployedBytecode = new Contract({
  name: "ContractWithDeployedBytecode",
}).withDeployedBytecode(bytecodeBytes);
type _ContractWithDeployedBytecodeGetter = Expect<
  Equal<
    (typeof contractWithDeployedBytecode)["deployedBytecode"],
    ContractBytecode
  >
>;

const contractWithDeployedBytecodeNull = new Contract({
  name: "ContractWithDeployedBytecodeNull",
}).withDeployedBytecode(null);
type _ContractWithDeployedBytecodeNullGetter = Expect<
  Equal<(typeof contractWithDeployedBytecodeNull)["deployedBytecode"], null>
>;

const contractWithDeployedBytecodeHex = new Contract({
  name: "ContractWithDeployedBytecodeHex",
}).withDeployedBytecode("0xfeedface");
type _ContractWithDeployedBytecodeHexGetter = Expect<
  Equal<
    (typeof contractWithDeployedBytecodeHex)["deployedBytecode"],
    ContractBytecode
  >
>;
type _ContractWithDeployedBytecodeHexState = Expect<
  Equal<
    ContractInstanceState<
      typeof contractWithDeployedBytecodeHex
    >["deployedBytecode"],
    ContractBytecode
  >
>;

const contractFromSolc = Contract.fromSolcContractOutput(
  "ContractFromSolc",
  {} as object
);
type ContractFromSolcInstance = typeof contractFromSolc;
type ContractFromSolcState = ContractInstanceState<ContractFromSolcInstance>;
type _ContractFromSolcAddressNoUndefined = Expect<
  Equal<NoUndefined<ContractFromSolcInstance["address"]>, true>
>;
type _ContractFromSolcAbiNoUndefined = Expect<
  Equal<NoUndefined<ContractFromSolcInstance["abi"]>, true>
>;
type _ContractFromSolcCreationBytecodeNoUndefined = Expect<
  Equal<NoUndefined<ContractFromSolcInstance["creationBytecode"]>, true>
>;
type _ContractFromSolcDeployedBytecodeNoUndefined = Expect<
  Equal<NoUndefined<ContractFromSolcInstance["deployedBytecode"]>, true>
>;
type _ContractFromSolcToJsonMatchesState = Expect<
  Equal<ReturnType<ContractFromSolcInstance["toJson"]>, ContractFromSolcState>
>;

const contractFromSolcWithMutation = contractFromSolc.withAddress(
  "0xbeef" as `0x${string}`
);
type _ContractFromSolcWithMutationGetter = Expect<
  Equal<(typeof contractFromSolcWithMutation)["address"], `0x${string}`>
>;
type _ContractFromSolcWithMutationState = Expect<
  Equal<
    ContractInstanceState<typeof contractFromSolcWithMutation>["address"],
    `0x${string}`
  >
>;
