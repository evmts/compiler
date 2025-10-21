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
  __napiInstance.exports['__napi_register__JsAst_impl_8']?.()
  __napiInstance.exports['__napi_register__SeverityLevel_9']?.()
  __napiInstance.exports['__napi_register__SourceLocation_struct_10']?.()
  __napiInstance.exports['__napi_register__SecondarySourceLocation_struct_11']?.()
  __napiInstance.exports['__napi_register__CompilerError_struct_12']?.()
  __napiInstance.exports['__napi_register__JsSourceArtifacts_struct_13']?.()
  __napiInstance.exports['__napi_register__JsSourceArtifacts_impl_21']?.()
  __napiInstance.exports['__napi_register__JsCompileOutput_struct_22']?.()
  __napiInstance.exports['__napi_register__JsCompileOutput_impl_30']?.()
  __napiInstance.exports['__napi_register__JsCompiler_struct_31']?.()
  __napiInstance.exports['__napi_register__JsCompiler_impl_44']?.()
  __napiInstance.exports['__napi_register__ImmutableSlot_struct_45']?.()
  __napiInstance.exports['__napi_register__JsFunctionDebugDataEntry_struct_46']?.()
  __napiInstance.exports['__napi_register__JsGasEstimatesCreation_struct_47']?.()
  __napiInstance.exports['__napi_register__JsGasEstimates_struct_48']?.()
  __napiInstance.exports['__napi_register__JsEwasm_struct_49']?.()
  __napiInstance.exports['__napi_register__JsContractBytecode_struct_50']?.()
  __napiInstance.exports['__napi_register__JsContractState_struct_51']?.()
  __napiInstance.exports['__napi_register__JsContract_struct_52']?.()
  __napiInstance.exports['__napi_register__JsContract_impl_79']?.()
  __napiInstance.exports['__napi_register__JsCompilerConfigOptions_struct_80']?.()
  __napiInstance.exports['__napi_register__JsAstConfigOptions_struct_81']?.()
  __napiInstance.exports['__napi_register__SolcLanguage_82']?.()
  __napiInstance.exports['__napi_register__ProjectPaths_struct_83']?.()
  __napiInstance.exports['__napi_register__JsCompilerSettingsOptions_struct_84']?.()
  __napiInstance.exports['__napi_register__JsOptimizerSettingsOptions_struct_85']?.()
  __napiInstance.exports['__napi_register__JsOptimizerDetailsOptions_struct_86']?.()
  __napiInstance.exports['__napi_register__JsYulDetailsOptions_struct_87']?.()
  __napiInstance.exports['__napi_register__JsDebuggingSettingsOptions_struct_88']?.()
  __napiInstance.exports['__napi_register__JsSettingsMetadataOptions_struct_89']?.()
  __napiInstance.exports['__napi_register__JsModelCheckerSettingsOptions_struct_90']?.()
  __napiInstance.exports['__napi_register__BytecodeHash_91']?.()
  __napiInstance.exports['__napi_register__RevertStrings_92']?.()
  __napiInstance.exports['__napi_register__ModelCheckerEngine_93']?.()
  __napiInstance.exports['__napi_register__ModelCheckerTarget_94']?.()
  __napiInstance.exports['__napi_register__ModelCheckerInvariant_95']?.()
  __napiInstance.exports['__napi_register__ModelCheckerSolver_96']?.()
  __napiInstance.exports['__napi_register__ModelCheckerInvariantKind_97']?.()
  __napiInstance.exports['__napi_register__EvmVersion_98']?.()
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
export const EvmVersion = __napiModule.exports.EvmVersion
export const ModelCheckerEngine = __napiModule.exports.ModelCheckerEngine
export const ModelCheckerInvariant = __napiModule.exports.ModelCheckerInvariant
export const ModelCheckerInvariantKind = __napiModule.exports.ModelCheckerInvariantKind
export const ModelCheckerSolver = __napiModule.exports.ModelCheckerSolver
export const ModelCheckerTarget = __napiModule.exports.ModelCheckerTarget
export const RevertStrings = __napiModule.exports.RevertStrings
export const SeverityLevel = __napiModule.exports.SeverityLevel
export const SolcLanguage = __napiModule.exports.SolcLanguage
