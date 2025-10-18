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
  __napiInstance.exports['__napi_register__CompilerError_struct_9']?.()
  __napiInstance.exports['__napi_register__SourceLocation_struct_10']?.()
  __napiInstance.exports['__napi_register__ContractBytecode_struct_11']?.()
  __napiInstance.exports['__napi_register__ContractArtifact_struct_12']?.()
  __napiInstance.exports['__napi_register__CompileOutput_struct_13']?.()
  __napiInstance.exports['__napi_register__JsCompiler_struct_14']?.()
  __napiInstance.exports['__napi_register__JsCompiler_impl_27']?.()
  __napiInstance.exports['__napi_register__JsCompilerConfigOptions_struct_28']?.()
  __napiInstance.exports['__napi_register__JsAstConfigOptions_struct_29']?.()
  __napiInstance.exports['__napi_register__SolcLanguage_30']?.()
  __napiInstance.exports['__napi_register__ProjectPaths_struct_31']?.()
  __napiInstance.exports['__napi_register__JsCompilerSettingsOptions_struct_32']?.()
  __napiInstance.exports['__napi_register__JsOptimizerSettingsOptions_struct_33']?.()
  __napiInstance.exports['__napi_register__JsOptimizerDetailsOptions_struct_34']?.()
  __napiInstance.exports['__napi_register__JsYulDetailsOptions_struct_35']?.()
  __napiInstance.exports['__napi_register__JsDebuggingSettingsOptions_struct_36']?.()
  __napiInstance.exports['__napi_register__JsSettingsMetadataOptions_struct_37']?.()
  __napiInstance.exports['__napi_register__JsModelCheckerSettingsOptions_struct_38']?.()
  __napiInstance.exports['__napi_register__BytecodeHash_39']?.()
  __napiInstance.exports['__napi_register__RevertStrings_40']?.()
  __napiInstance.exports['__napi_register__ModelCheckerEngine_41']?.()
  __napiInstance.exports['__napi_register__ModelCheckerTarget_42']?.()
  __napiInstance.exports['__napi_register__ModelCheckerInvariant_43']?.()
  __napiInstance.exports['__napi_register__ModelCheckerSolver_44']?.()
  __napiInstance.exports['__napi_register__ModelCheckerInvariantKind_45']?.()
  __napiInstance.exports['__napi_register__EvmVersion_46']?.()
}
export const Ast = __napiModule.exports.Ast
export const JsAst = __napiModule.exports.JsAst
export const Compiler = __napiModule.exports.Compiler
export const JsCompiler = __napiModule.exports.JsCompiler
export const BytecodeHash = __napiModule.exports.BytecodeHash
export const EvmVersion = __napiModule.exports.EvmVersion
export const ModelCheckerEngine = __napiModule.exports.ModelCheckerEngine
export const ModelCheckerInvariant = __napiModule.exports.ModelCheckerInvariant
export const ModelCheckerInvariantKind = __napiModule.exports.ModelCheckerInvariantKind
export const ModelCheckerSolver = __napiModule.exports.ModelCheckerSolver
export const ModelCheckerTarget = __napiModule.exports.ModelCheckerTarget
export const RevertStrings = __napiModule.exports.RevertStrings
export const SolcLanguage = __napiModule.exports.SolcLanguage
