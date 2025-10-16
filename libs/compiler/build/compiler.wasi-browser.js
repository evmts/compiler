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
  __napiInstance.exports['__napi_register__Compiler_impl_8']?.()
  __napiInstance.exports['__napi_register__Instrument_struct_9']?.()
  __napiInstance.exports['__napi_register__Instrument_impl_18']?.()
  __napiInstance.exports['__napi_register__CompilerOptions_struct_19']?.()
  __napiInstance.exports['__napi_register__InstrumentOptions_struct_20']?.()
  __napiInstance.exports['__napi_register__CompilerSettings_struct_21']?.()
  __napiInstance.exports['__napi_register__OptimizerSettings_struct_22']?.()
  __napiInstance.exports['__napi_register__OptimizerDetails_struct_23']?.()
  __napiInstance.exports['__napi_register__YulDetails_struct_24']?.()
  __napiInstance.exports['__napi_register__DebuggingSettings_struct_25']?.()
  __napiInstance.exports['__napi_register__SettingsMetadata_struct_26']?.()
  __napiInstance.exports['__napi_register__ModelCheckerSettings_struct_27']?.()
  __napiInstance.exports['__napi_register__BytecodeHash_28']?.()
  __napiInstance.exports['__napi_register__RevertStrings_29']?.()
  __napiInstance.exports['__napi_register__ModelCheckerEngine_30']?.()
  __napiInstance.exports['__napi_register__ModelCheckerTarget_31']?.()
  __napiInstance.exports['__napi_register__ModelCheckerInvariant_32']?.()
  __napiInstance.exports['__napi_register__ModelCheckerSolver_33']?.()
  __napiInstance.exports['__napi_register__EvmVersion_34']?.()
  __napiInstance.exports['__napi_register__SolidityProjectBuilder_struct_35']?.()
  __napiInstance.exports['__napi_register__SolidityProjectBuilder_impl_48']?.()
  __napiInstance.exports['__napi_register__SolidityProject_struct_49']?.()
  __napiInstance.exports['__napi_register__SolidityProject_impl_57']?.()
  __napiInstance.exports['__napi_register__create_hardhat_paths_58']?.()
  __napiInstance.exports['__napi_register__create_dapptools_paths_59']?.()
  __napiInstance.exports['__napi_register__create_current_hardhat_paths_60']?.()
  __napiInstance.exports['__napi_register__create_current_dapptools_paths_61']?.()
  __napiInstance.exports['__napi_register__find_artifacts_dir_62']?.()
  __napiInstance.exports['__napi_register__find_source_dir_63']?.()
  __napiInstance.exports['__napi_register__find_libs_64']?.()
  __napiInstance.exports['__napi_register__CompilerError_struct_65']?.()
  __napiInstance.exports['__napi_register__SourceLocation_struct_66']?.()
  __napiInstance.exports['__napi_register__ContractArtifact_struct_67']?.()
  __napiInstance.exports['__napi_register__CompileOutput_struct_68']?.()
  __napiInstance.exports['__napi_register__ProjectPaths_struct_69']?.()
}
export const Compiler = __napiModule.exports.Compiler
export const Instrument = __napiModule.exports.Instrument
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
