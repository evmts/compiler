import type { CompiledSource } from '../internal/CompiledSource.js'
import type { CompilationOutputOption } from './CompilationOutputOption.js'
import type { CompileBaseResult } from './CompileBaseResult.js'

export interface CompileSourceResult<
	TCompilationOutput extends CompilationOutputOption[] | undefined = CompilationOutputOption[] | undefined,
> extends CompileBaseResult {
	compilationResult: CompiledSource<TCompilationOutput>
}
