import type {
  Compiler,
  CompilerError,
  SourceArtifacts,
} from "../build/index.js";

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B
  ? 1
  : 2
  ? true
  : false;
type Expect<T extends true> = T;

declare const compiler: Compiler;
declare const contract: import("../build/index.js").Contract;

/* ------------------------------ compileFiles ------------------------------ */
const singlePathResult = compiler.compileFiles(["contracts/Only.sol"] as const);
if (singlePathResult.hasCompilerErrors()) {
  type _SinglePathResultArtifacts = Expect<
    Equal<
      (typeof singlePathResult)["artifacts"],
      { readonly "contracts/Only.sol"?: SourceArtifacts<"contracts/Only.sol"> }
    >
  >;
  type _SinglePathResultArtifact = Expect<
    Equal<(typeof singlePathResult)["artifact"], never>
  >;
  type _SinglePathResultErrors = Expect<
    Equal<(typeof singlePathResult)["errors"], ReadonlyArray<CompilerError>>
  >;
} else {
  type _SinglePathResultArtifacts = Expect<
    Equal<
      (typeof singlePathResult)["artifacts"],
      { readonly "contracts/Only.sol": SourceArtifacts<"contracts/Only.sol"> }
    >
  >;
  type _SinglePathResultArtifact = Expect<
    Equal<(typeof singlePathResult)["artifact"], never>
  >;
  type _SinglePathResultErrors = Expect<
    Equal<(typeof singlePathResult)["errors"], undefined>
  >;
}

const multiPathResult = compiler.compileFiles([
  "contracts/A.sol",
  "contracts/B.sol",
] as const);
if (multiPathResult.hasCompilerErrors()) {
  type _MultiPathResultArtifacts = Expect<
    Equal<
      (typeof multiPathResult)["artifacts"],
      {
        readonly "contracts/A.sol"?: SourceArtifacts<"contracts/A.sol">;
        readonly "contracts/B.sol"?: SourceArtifacts<"contracts/B.sol">;
      }
    >
  >;
  type _MultiPathResultArtifact = Expect<
    Equal<(typeof multiPathResult)["artifact"], never>
  >;
  type _MultiPathResultErrors = Expect<
    Equal<(typeof multiPathResult)["errors"], ReadonlyArray<CompilerError>>
  >;
} else {
  type _MultiPathResultArtifacts = Expect<
    Equal<
      (typeof multiPathResult)["artifacts"],
      {
        readonly "contracts/A.sol": SourceArtifacts<"contracts/A.sol">;
        readonly "contracts/B.sol": SourceArtifacts<"contracts/B.sol">;
      }
    >
  >;
  type _MultiPathResultArtifact = Expect<
    Equal<(typeof multiPathResult)["artifact"], never>
  >;
  type _MultiPathResultErrors = Expect<
    Equal<(typeof multiPathResult)["errors"], undefined>
  >;
}

/* ----------------------------- compileSources ----------------------------- */
const sourcesResult = compiler.compileSources({
  ["Foo.sol"]: "contract Foo {}",
  ["Bar.sol"]: "contract Bar {}",
} as const);
if (sourcesResult.hasCompilerErrors()) {
  type _SourcesResultArtifacts = Expect<
    Equal<
      (typeof sourcesResult)["artifacts"],
      {
        readonly "Foo.sol"?: SourceArtifacts<"Foo.sol">;
        readonly "Bar.sol"?: SourceArtifacts<"Bar.sol">;
      }
    >
  >;
  type _SourcesResultArtifact = Expect<
    Equal<(typeof sourcesResult)["artifact"], never>
  >;
  type _SourcesResultErrors = Expect<
    Equal<(typeof sourcesResult)["errors"], ReadonlyArray<CompilerError>>
  >;
} else {
  type _SourcesResultArtifacts = Expect<
    Equal<
      (typeof sourcesResult)["artifacts"],
      {
        readonly "Foo.sol": SourceArtifacts<"Foo.sol">;
        readonly "Bar.sol": SourceArtifacts<"Bar.sol">;
      }
    >
  >;
  type _SourcesResultArtifact = Expect<
    Equal<(typeof sourcesResult)["artifact"], never>
  >;
  type _SourcesResultErrors = Expect<
    Equal<(typeof sourcesResult)["errors"], undefined>
  >;
}

/* ------------------------------ compileSource ----------------------------- */
const singleSourceResult = compiler.compileSource("contract Foo { }");
if (singleSourceResult.hasCompilerErrors()) {
  type _SingleSourceResultArtifacts = Expect<
    Equal<(typeof singleSourceResult)["artifacts"], never>
  >;
  type _SingleSourceResultArtifact = Expect<
    Equal<(typeof singleSourceResult)["artifact"], SourceArtifacts>
  >;
  type _SingleSourceResultErrors = Expect<
    Equal<(typeof singleSourceResult)["errors"], ReadonlyArray<CompilerError>>
  >;
} else {
  type _SingleSourceResultArtifacts = Expect<
    Equal<(typeof singleSourceResult)["artifacts"], never>
  >;
  type _SingleSourceResultArtifact = Expect<
    Equal<(typeof singleSourceResult)["artifact"], SourceArtifacts>
  >;
  type _SingleSourceResultErrors = Expect<
    Equal<(typeof singleSourceResult)["errors"], undefined>
  >;
}

/* ----------------------------- compileContract ---------------------------- */
const singleContractResult = compiler.compileContract("Foo");
if (singleContractResult.hasCompilerErrors()) {
  type _SingleContractResultArtifacts = Expect<
    Equal<(typeof singleContractResult)["artifacts"], never>
  >;
  type _SingleContractResultArtifact = Expect<
    Equal<(typeof singleContractResult)["artifact"], SourceArtifacts>
  >;
  type _SingleContractResultErrors = Expect<
    Equal<(typeof singleContractResult)["errors"], ReadonlyArray<CompilerError>>
  >;
} else {
  type _SingleContractResultArtifacts = Expect<
    Equal<(typeof singleContractResult)["artifacts"], never>
  >;
  type _SingleContractResultArtifact = Expect<
    Equal<(typeof singleContractResult)["artifact"], SourceArtifacts>
  >;
  type _SingleContractResultErrors = Expect<
    Equal<(typeof singleContractResult)["errors"], undefined>
  >;
}

/* ----------------------------- compileProject ----------------------------- */
const projectResult = compiler.compileProject();
if (projectResult.hasCompilerErrors()) {
  type _ProjectResultArtifacts = Expect<
    Equal<
      (typeof projectResult)["artifacts"],
      Readonly<Partial<Record<string, SourceArtifacts>>>
    >
  >;
  type _ProjectResultArtifact = Expect<
    Equal<(typeof projectResult)["artifact"], never>
  >;
  type _ProjectResultErrors = Expect<
    Equal<(typeof projectResult)["errors"], ReadonlyArray<CompilerError>>
  >;
} else {
  type _ProjectResultArtifacts = Expect<
    Equal<
      (typeof projectResult)["artifacts"],
      Readonly<Record<string, SourceArtifacts>>
    >
  >;
  type _ProjectResultArtifact = Expect<
    Equal<(typeof projectResult)["artifact"], never>
  >;
  type _ProjectResultErrors = Expect<
    Equal<(typeof projectResult)["errors"], undefined>
  >;
}
export {};
