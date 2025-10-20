import type {
  CompileOutput,
  CompilerError,
  SourceArtifacts,
} from "../build/index.js";

type Expect<T extends true> = T;
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B
  ? 1
  : 2
  ? true
  : false;

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
