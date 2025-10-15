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
  __napiInstance.exports['__napi_register__SolidityProjectBuilder_struct_0']?.()
  __napiInstance.exports['__napi_register__SolidityProjectBuilder_impl_13']?.()
  __napiInstance.exports['__napi_register__SolidityProject_struct_14']?.()
  __napiInstance.exports['__napi_register__SolidityProject_impl_22']?.()
  __napiInstance.exports['__napi_register__create_hardhat_paths_23']?.()
  __napiInstance.exports['__napi_register__create_dapptools_paths_24']?.()
  __napiInstance.exports['__napi_register__create_current_hardhat_paths_25']?.()
  __napiInstance.exports['__napi_register__create_current_dapptools_paths_26']?.()
  __napiInstance.exports['__napi_register__sum_27']?.()
  __napiInstance.exports['__napi_register__find_artifacts_dir_28']?.()
  __napiInstance.exports['__napi_register__find_source_dir_29']?.()
  __napiInstance.exports['__napi_register__find_libs_30']?.()
  __napiInstance.exports['__napi_register__Shadow_struct_31']?.()
  __napiInstance.exports['__napi_register__Shadow_impl_36']?.()
  __napiInstance.exports['__napi_register__CompilerError_struct_37']?.()
  __napiInstance.exports['__napi_register__SourceLocation_struct_38']?.()
  __napiInstance.exports['__napi_register__ContractArtifact_struct_39']?.()
  __napiInstance.exports['__napi_register__CompileOutput_struct_40']?.()
  __napiInstance.exports['__napi_register__ProjectPaths_struct_41']?.()
}
export const Shadow = __napiModule.exports.Shadow
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
