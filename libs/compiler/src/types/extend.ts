import { Ast, CompilerError, Contract } from "../../build";

type WithPathKey<TPath, TValue> = TValue extends SourceArtifacts<infer _>
  ? SourceArtifacts<Extract<TPath, string>>
  : TValue;

type ReadonlyRecord<K extends PropertyKey, V> = Readonly<
  { [P in K]: WithPathKey<P, V> }
>;

type ReadonlyPartialRecord<K extends PropertyKey, V> = Readonly<
  Partial<{ [P in K]: WithPathKey<P, V> }>
>;

type ArtifactMap<
  THasErrors extends boolean,
  TPaths extends readonly string[] | undefined,
> = TPaths extends readonly string[]
  ? THasErrors extends false
    ? ReadonlyRecord<TPaths[number], SourceArtifacts>
    : ReadonlyPartialRecord<TPaths[number], SourceArtifacts>
  : never;

type ArtifactValue<
  THasErrors extends boolean,
  TPaths extends readonly string[] | undefined,
> = TPaths extends undefined
  ? THasErrors extends false
    ? SourceArtifacts
    : SourceArtifacts | undefined
  : never;

/**
 * Place custom type/interface overrides here. The postbuild-dts script will
 * replace matching declarations in build/index.d.ts after every build.
 */
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
