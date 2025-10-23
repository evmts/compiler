import {
  instantiateNapiModuleSync as __emnapiInstantiateNapiModuleSync,
  getDefaultContext as __emnapiGetDefaultContext,
  WASI as __WASI,
  createOnMessage as __wasmCreateOnMessageForFsProxy,
} from '@napi-rs/wasm-runtime'

import __wasmUrl from './compiler.wasm32-wasi.wasm?url'

const __wasi = new __WASI({
  version: 'preview1',
})

const __emnapiContext = __emnapiGetDefaultContext()

const __sharedMemory = new WebAssembly.Memory({
  initial: 4000,
  maximum: 65536,
  shared: true,
})

const __wasmFile = await fetch(__wasmUrl).then((res) => res.arrayBuffer())

const {
  instance: __napiInstance,
  module: __wasiModule,
  napiModule: __napiModule,
} = __emnapiInstantiateNapiModuleSync(__wasmFile, {
  context: __emnapiContext,
  asyncWorkPoolSize: 4,
  wasi: __wasi,
  onCreateWorker() {
    const worker = new Worker(new URL('./wasi-worker-browser.mjs', import.meta.url), {
      type: 'module',
    })

    return worker
  },
  overwriteImports(importObject) {
    importObject.env = {
      ...importObject.env,
      ...importObject.napi,
      ...importObject.emnapi,
      memory: __sharedMemory,
    }
    return importObject
  },
  beforeInit({ instance }) {
    __napi_rs_initialize_modules(instance)
  },
})

function __napi_rs_initialize_modules(__napiInstance) {
  __napiInstance.exports['__napi_register__JsAst_struct_0']?.()
  __napiInstance.exports['__napi_register__JsAst_impl_10']?.()
  __napiInstance.exports['__napi_register__SeverityLevel_11']?.()
  __napiInstance.exports['__napi_register__SourceLocation_struct_12']?.()
  __napiInstance.exports['__napi_register__SecondarySourceLocation_struct_13']?.()
  __napiInstance.exports['__napi_register__VyperSourceLocation_struct_14']?.()
  __napiInstance.exports['__napi_register__CompilerError_struct_15']?.()
  __napiInstance.exports['__napi_register__SourceArtifactsJson_struct_16']?.()
  __napiInstance.exports['__napi_register__CompileOutputJson_struct_17']?.()
  __napiInstance.exports['__napi_register__JsSourceArtifacts_struct_18']?.()
  __napiInstance.exports['__napi_register__JsSourceArtifacts_impl_26']?.()
  __napiInstance.exports['__napi_register__JsCompileOutput_struct_27']?.()
  __napiInstance.exports['__napi_register__JsCompileOutput_impl_36']?.()
  __napiInstance.exports['__napi_register__JsCompiler_struct_37']?.()
  __napiInstance.exports['__napi_register__JsCompiler_impl_50']?.()
  __napiInstance.exports['__napi_register__ImmutableSlot_struct_51']?.()
  __napiInstance.exports['__napi_register__JsFunctionDebugDataEntry_struct_52']?.()
  __napiInstance.exports['__napi_register__JsGasEstimatesCreation_struct_53']?.()
  __napiInstance.exports['__napi_register__JsGasEstimates_struct_54']?.()
  __napiInstance.exports['__napi_register__JsEwasm_struct_55']?.()
  __napiInstance.exports['__napi_register__JsContractBytecode_struct_56']?.()
  __napiInstance.exports['__napi_register__JsContractState_struct_57']?.()
  __napiInstance.exports['__napi_register__JsContract_struct_58']?.()
  __napiInstance.exports['__napi_register__JsContract_impl_85']?.()
  __napiInstance.exports['__napi_register__JsCompilerConfigOptions_struct_86']?.()
  __napiInstance.exports['__napi_register__JsCompilerLanguage_87']?.()
  __napiInstance.exports['__napi_register__JsLoggingLevel_88']?.()
  __napiInstance.exports['__napi_register__JsVyperOptimizationMode_89']?.()
  __napiInstance.exports['__napi_register__JsVyperCompilerConfig_struct_90']?.()
  __napiInstance.exports['__napi_register__JsAstConfigOptions_struct_91']?.()
  __napiInstance.exports['__napi_register__SolcLanguage_92']?.()
  __napiInstance.exports['__napi_register__JsResolveConflictStrategy_93']?.()
  __napiInstance.exports['__napi_register__ProjectPaths_struct_94']?.()
  __napiInstance.exports['__napi_register__JsCompilerSettingsOptions_struct_95']?.()
  __napiInstance.exports['__napi_register__JsOptimizerSettingsOptions_struct_96']?.()
  __napiInstance.exports['__napi_register__JsOptimizerDetailsOptions_struct_97']?.()
  __napiInstance.exports['__napi_register__JsYulDetailsOptions_struct_98']?.()
  __napiInstance.exports['__napi_register__JsDebuggingSettingsOptions_struct_99']?.()
  __napiInstance.exports['__napi_register__JsSettingsMetadataOptions_struct_100']?.()
  __napiInstance.exports['__napi_register__JsModelCheckerSettingsOptions_struct_101']?.()
  __napiInstance.exports['__napi_register__BytecodeHash_102']?.()
  __napiInstance.exports['__napi_register__RevertStrings_103']?.()
  __napiInstance.exports['__napi_register__ModelCheckerEngine_104']?.()
  __napiInstance.exports['__napi_register__ModelCheckerTarget_105']?.()
  __napiInstance.exports['__napi_register__ModelCheckerInvariant_106']?.()
  __napiInstance.exports['__napi_register__ModelCheckerSolver_107']?.()
  __napiInstance.exports['__napi_register__ModelCheckerInvariantKind_108']?.()
  __napiInstance.exports['__napi_register__EvmVersion_109']?.()
}
export const Ast = __napiModule.exports.Ast
export const JsAst = __napiModule.exports.JsAst
export const CompileOutput = __napiModule.exports.CompileOutput
export const JsCompileOutput = __napiModule.exports.JsCompileOutput
export const Compiler = __napiModule.exports.Compiler
export const JsCompiler = __napiModule.exports.JsCompiler
export const Contract = __napiModule.exports.Contract
export const JsContract = __napiModule.exports.JsContract
export const SourceArtifacts = __napiModule.exports.SourceArtifacts
export const JsSourceArtifacts = __napiModule.exports.JsSourceArtifacts
export const BytecodeHash = __napiModule.exports.BytecodeHash
export const CompilerLanguage = __napiModule.exports.CompilerLanguage
export const JsCompilerLanguage = __napiModule.exports.JsCompilerLanguage
export const EvmVersion = __napiModule.exports.EvmVersion
export const LoggingLevel = __napiModule.exports.LoggingLevel
export const JsLoggingLevel = __napiModule.exports.JsLoggingLevel
export const ModelCheckerEngine = __napiModule.exports.ModelCheckerEngine
export const ModelCheckerInvariant = __napiModule.exports.ModelCheckerInvariant
export const ModelCheckerInvariantKind = __napiModule.exports.ModelCheckerInvariantKind
export const ModelCheckerSolver = __napiModule.exports.ModelCheckerSolver
export const ModelCheckerTarget = __napiModule.exports.ModelCheckerTarget
export const ResolveConflictStrategy = __napiModule.exports.ResolveConflictStrategy
export const JsResolveConflictStrategy = __napiModule.exports.JsResolveConflictStrategy
export const RevertStrings = __napiModule.exports.RevertStrings
export const SeverityLevel = __napiModule.exports.SeverityLevel
export const SolcLanguage = __napiModule.exports.SolcLanguage
export const VyperOptimizationMode = __napiModule.exports.VyperOptimizationMode
export const JsVyperOptimizationMode = __napiModule.exports.JsVyperOptimizationMode
