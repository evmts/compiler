import type { CompileContractsResult } from '../internal/CompileContractsResult.js'
import type { CompilationOutputOption } from './CompilationOutputOption.js'

export interface CompileFilesResult<
	TCompilationOutput extends CompilationOutputOption[] | undefined = CompilationOutputOption[] | undefined,
	TFilePaths extends string[] = string[],
> extends CompileContractsResult<TCompilationOutput, TFilePaths> {}
