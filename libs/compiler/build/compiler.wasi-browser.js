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
  __napiInstance.exports['__napi_register__SourceLocation_struct_11']?.()
  __napiInstance.exports['__napi_register__SecondarySourceLocation_struct_12']?.()
  __napiInstance.exports['__napi_register__VyperSourceLocation_struct_13']?.()
  __napiInstance.exports['__napi_register__CompilerError_struct_14']?.()
  __napiInstance.exports['__napi_register__SourceArtifactsJson_struct_15']?.()
  __napiInstance.exports['__napi_register__CompileOutputJson_struct_16']?.()
  __napiInstance.exports['__napi_register__JsSourceArtifacts_struct_17']?.()
  __napiInstance.exports['__napi_register__JsSourceArtifacts_impl_25']?.()
  __napiInstance.exports['__napi_register__JsCompileOutput_struct_26']?.()
  __napiInstance.exports['__napi_register__JsCompileOutput_impl_35']?.()
  __napiInstance.exports['__napi_register__JsCompiler_struct_36']?.()
  __napiInstance.exports['__napi_register__JsCompiler_impl_49']?.()
  __napiInstance.exports['__napi_register__ImmutableSlot_struct_50']?.()
  __napiInstance.exports['__napi_register__JsFunctionDebugDataEntry_struct_51']?.()
  __napiInstance.exports['__napi_register__JsGasEstimatesCreation_struct_52']?.()
  __napiInstance.exports['__napi_register__JsGasEstimates_struct_53']?.()
  __napiInstance.exports['__napi_register__JsEwasm_struct_54']?.()
  __napiInstance.exports['__napi_register__JsContractBytecode_struct_55']?.()
  __napiInstance.exports['__napi_register__JsContractState_struct_56']?.()
  __napiInstance.exports['__napi_register__JsContract_struct_57']?.()
  __napiInstance.exports['__napi_register__JsContract_impl_84']?.()
  __napiInstance.exports['__napi_register__JsCompilerConfigOptions_struct_85']?.()
  __napiInstance.exports['__napi_register__JsVyperCompilerConfig_struct_86']?.()
  __napiInstance.exports['__napi_register__JsAstConfigOptions_struct_87']?.()
  __napiInstance.exports['__napi_register__ProjectPaths_struct_88']?.()
  __napiInstance.exports['__napi_register__JsCompilerSettingsOptions_struct_89']?.()
  __napiInstance.exports['__napi_register__JsOptimizerSettingsOptions_struct_90']?.()
  __napiInstance.exports['__napi_register__JsOptimizerDetailsOptions_struct_91']?.()
  __napiInstance.exports['__napi_register__JsYulDetailsOptions_struct_92']?.()
  __napiInstance.exports['__napi_register__JsDebuggingSettingsOptions_struct_93']?.()
  __napiInstance.exports['__napi_register__JsSettingsMetadataOptions_struct_94']?.()
  __napiInstance.exports['__napi_register__JsModelCheckerSettingsOptions_struct_95']?.()
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
