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
  __napiInstance.exports['__napi_register__ProjectPaths_struct_0']?.()
  __napiInstance.exports['__napi_register__create_hardhat_paths_1']?.()
  __napiInstance.exports['__napi_register__create_dapptools_paths_2']?.()
  __napiInstance.exports['__napi_register__create_current_hardhat_paths_3']?.()
  __napiInstance.exports['__napi_register__create_current_dapptools_paths_4']?.()
  __napiInstance.exports['__napi_register__CompilerError_struct_5']?.()
  __napiInstance.exports['__napi_register__SourceLocation_struct_6']?.()
  __napiInstance.exports['__napi_register__ContractArtifact_struct_7']?.()
  __napiInstance.exports['__napi_register__CompileOutput_struct_8']?.()
  __napiInstance.exports['__napi_register__SolidityProject_struct_9']?.()
  __napiInstance.exports['__napi_register__SolidityProject_impl_17']?.()
  __napiInstance.exports['__napi_register__SolidityProjectBuilder_struct_18']?.()
  __napiInstance.exports['__napi_register__SolidityProjectBuilder_impl_31']?.()
  __napiInstance.exports['__napi_register__sum_32']?.()
  __napiInstance.exports['__napi_register__find_artifacts_dir_33']?.()
  __napiInstance.exports['__napi_register__find_source_dir_34']?.()
  __napiInstance.exports['__napi_register__find_libs_35']?.()
}
export const SolidityProject = __napiModule.exports.SolidityProject
export const SolidityProjectBuilder = __napiModule.exports.SolidityProjectBuilder
export const createCurrentDapptoolsPaths = __napiModule.exports.createCurrentDapptoolsPaths
export const createCurrentHardhatPaths = __napiModule.exports.createCurrentHardhatPaths
export const createDapptoolsPaths = __napiModule.exports.createDapptoolsPaths
export const createHardhatPaths = __napiModule.exports.createHardhatPaths
export const findArtifactsDir = __napiModule.exports.findArtifactsDir
export const findLibs = __napiModule.exports.findLibs
export const findSourceDir = __napiModule.exports.findSourceDir
export const sum = __napiModule.exports.sum
