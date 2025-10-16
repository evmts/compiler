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
  __napiInstance.exports['__napi_register__Compiler_struct_0']?.()
  __napiInstance.exports['__napi_register__Compiler_impl_6']?.()
  __napiInstance.exports['__napi_register__CompilerOptions_struct_7']?.()
  __napiInstance.exports['__napi_register__ShadowOptions_struct_8']?.()
  __napiInstance.exports['__napi_register__CompilerSettings_struct_9']?.()
  __napiInstance.exports['__napi_register__OptimizerSettings_struct_10']?.()
  __napiInstance.exports['__napi_register__OptimizerDetails_struct_11']?.()
  __napiInstance.exports['__napi_register__YulDetails_struct_12']?.()
  __napiInstance.exports['__napi_register__DebuggingSettings_struct_13']?.()
  __napiInstance.exports['__napi_register__SettingsMetadata_struct_14']?.()
  __napiInstance.exports['__napi_register__ModelCheckerSettings_struct_15']?.()
  __napiInstance.exports['__napi_register__BytecodeHash_16']?.()
  __napiInstance.exports['__napi_register__RevertStrings_17']?.()
  __napiInstance.exports['__napi_register__ModelCheckerEngine_18']?.()
  __napiInstance.exports['__napi_register__ModelCheckerTarget_19']?.()
  __napiInstance.exports['__napi_register__ModelCheckerInvariant_20']?.()
  __napiInstance.exports['__napi_register__ModelCheckerSolver_21']?.()
  __napiInstance.exports['__napi_register__EvmVersion_22']?.()
  __napiInstance.exports['__napi_register__SolidityProjectBuilder_struct_23']?.()
  __napiInstance.exports['__napi_register__SolidityProjectBuilder_impl_36']?.()
  __napiInstance.exports['__napi_register__SolidityProject_struct_37']?.()
  __napiInstance.exports['__napi_register__SolidityProject_impl_45']?.()
  __napiInstance.exports['__napi_register__create_hardhat_paths_46']?.()
  __napiInstance.exports['__napi_register__create_dapptools_paths_47']?.()
  __napiInstance.exports['__napi_register__create_current_hardhat_paths_48']?.()
  __napiInstance.exports['__napi_register__create_current_dapptools_paths_49']?.()
  __napiInstance.exports['__napi_register__find_artifacts_dir_50']?.()
  __napiInstance.exports['__napi_register__find_source_dir_51']?.()
  __napiInstance.exports['__napi_register__find_libs_52']?.()
  __napiInstance.exports['__napi_register__Shadow_struct_53']?.()
  __napiInstance.exports['__napi_register__Shadow_impl_57']?.()
  __napiInstance.exports['__napi_register__CompilerError_struct_58']?.()
  __napiInstance.exports['__napi_register__SourceLocation_struct_59']?.()
  __napiInstance.exports['__napi_register__ContractArtifact_struct_60']?.()
  __napiInstance.exports['__napi_register__CompileOutput_struct_61']?.()
  __napiInstance.exports['__napi_register__ProjectPaths_struct_62']?.()
}
export const Compiler = __napiModule.exports.Compiler
export const Shadow = __napiModule.exports.Shadow
export const SolidityProject = __napiModule.exports.SolidityProject
export const SolidityProjectBuilder = __napiModule.exports.SolidityProjectBuilder
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
export const ModelCheckerSolver = __napiModule.exports.ModelCheckerSolver
export const ModelCheckerTarget = __napiModule.exports.ModelCheckerTarget
export const RevertStrings = __napiModule.exports.RevertStrings
