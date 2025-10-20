import { Ast, CompilerError, Contract } from "../../build";

/**
 * Place custom type/interface overrides here. The postbuild-dts script will
 * replace matching declarations in build/index.d.ts after every build.
 */
// TODO: type that if THasErrors is false then ALL fields in artifacts are defined
export declare class CompileOutput<
  THasErrors extends boolean = boolean,
  TSourcePaths extends readonly string[] | undefined = string[] | undefined
> {
  constructor();
  get artifactsJson(): Record<string, unknown>;
  get artifacts(): TSourcePaths extends readonly string[]
    ? THasErrors extends false
      ? { readonly [K in TSourcePaths[number]]: SourceArtifacts<K> }
      : { readonly [K in TSourcePaths[number]]?: SourceArtifacts<K> }
    : never;
  get artifact(): TSourcePaths extends undefined ? SourceArtifacts : never;
  get errors(): THasErrors extends true
    ? ReadonlyArray<CompilerError>
    : undefined;
  get diagnostics(): Array<CompilerError>;
  hasCompilerErrors(): this is CompileOutput<true, TSourcePaths>;
}

export declare class SourceArtifacts<TPath extends string = string> {
  constructor()
  get sourcePath(): TPath | null
  get sourceId(): number | null
  get solcVersion(): string | null
  get astJson(): import('./solc-ast').SourceUnit | undefined
  get ast(): Ast | undefined
  get contracts(): Record<string, Contract>
}