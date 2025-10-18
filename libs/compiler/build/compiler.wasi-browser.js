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
  __napiInstance.exports['__napi_register__JsCompiler_impl_25']?.()
  __napiInstance.exports['__napi_register__CompilerConfig_struct_26']?.()
  __napiInstance.exports['__napi_register__AstOptions_struct_27']?.()
  __napiInstance.exports['__napi_register__SolcLanguage_28']?.()
  __napiInstance.exports['__napi_register__ProjectPaths_struct_29']?.()
  __napiInstance.exports['__napi_register__create_hardhat_paths_30']?.()
  __napiInstance.exports['__napi_register__create_dapptools_paths_31']?.()
  __napiInstance.exports['__napi_register__create_current_hardhat_paths_32']?.()
  __napiInstance.exports['__napi_register__create_current_dapptools_paths_33']?.()
  __napiInstance.exports['__napi_register__find_artifacts_dir_34']?.()
  __napiInstance.exports['__napi_register__find_source_dir_35']?.()
  __napiInstance.exports['__napi_register__find_libs_36']?.()
  __napiInstance.exports['__napi_register__CompilerSettings_struct_37']?.()
  __napiInstance.exports['__napi_register__OptimizerSettings_struct_38']?.()
  __napiInstance.exports['__napi_register__OptimizerDetails_struct_39']?.()
  __napiInstance.exports['__napi_register__YulDetails_struct_40']?.()
  __napiInstance.exports['__napi_register__DebuggingSettings_struct_41']?.()
  __napiInstance.exports['__napi_register__SettingsMetadata_struct_42']?.()
  __napiInstance.exports['__napi_register__ModelCheckerSettings_struct_43']?.()
  __napiInstance.exports['__napi_register__BytecodeHash_44']?.()
  __napiInstance.exports['__napi_register__RevertStrings_45']?.()
  __napiInstance.exports['__napi_register__ModelCheckerEngine_46']?.()
  __napiInstance.exports['__napi_register__ModelCheckerTarget_47']?.()
  __napiInstance.exports['__napi_register__ModelCheckerInvariant_48']?.()
  __napiInstance.exports['__napi_register__ModelCheckerSolver_49']?.()
  __napiInstance.exports['__napi_register__ModelCheckerTargetType_50']?.()
  __napiInstance.exports['__napi_register__EvmVersion_51']?.()
  __napiInstance.exports['__napi_register__ModelCheckerInvariantKind_52']?.()
}
export const Ast = __napiModule.exports.Ast
export const JsAst = __napiModule.exports.JsAst
export const Compiler = __napiModule.exports.Compiler
export const JsCompiler = __napiModule.exports.JsCompiler
export const BytecodeHash = __napiModule.exports.BytecodeHash
export const createCurrentDapptoolsPaths = __napiModule.exports.createCurrentDapptoolsPaths
export const createCurrentHardhatPaths = __napiModule.exports.createCurrentHardhatPaths
export const createDapptoolsPaths = __napiModule.exports.createDapptoolsPaths
export const createHardhatPaths = __napiModule.exports.createHardhatPaths
export const EvmVersion = __napiModule.exports.EvmVersion
export const findArtifactsDir = __napiModule.exports.findArtifactsDir
export const findLibs = __napiModule.exports.findLibs
export const findSourceDir = __napiModule.exports.findSourceDir
export const ModelCheckerEngine = __napiModule.exports.ModelCheckerEngine
export const ModelCheckerInvariant = __napiModule.exports.ModelCheckerInvariant
export const ModelCheckerInvariantKind = __napiModule.exports.ModelCheckerInvariantKind
export const ModelCheckerSolver = __napiModule.exports.ModelCheckerSolver
export const ModelCheckerTarget = __napiModule.exports.ModelCheckerTarget
export const ModelCheckerTargetType = __napiModule.exports.ModelCheckerTargetType
export const RevertStrings = __napiModule.exports.RevertStrings
export const SolcLanguage = __napiModule.exports.SolcLanguage
