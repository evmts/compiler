/**
 * Shadow - TypeScript bindings for Solidity parser WASM module
 *
 * This module provides a type-safe wrapper around the Shadow Solidity parser,
 * which is compiled from Zig + C++ to WebAssembly using Emscripten.
 */

// Import the Emscripten module factory and generated types
import { SourceUnit } from 'solidity-ast';
import createShadowModule, { type MainModule, type Shadow as ShadowNative } from '../wasm/shadow.js';

let wasmModule: MainModule | null = null;
let initPromise: Promise<MainModule> | null = null;

/**
 * Shadow instance wrapper that manages the lifecycle of a native Shadow object
 */
export class Shadow {
  private native: ShadowNative | null = null;

  /**
   * Private constructor - use Shadow.create() instead
   */
  private constructor(native: ShadowNative) {
    this.native = native;
  }

  /**
   * Initialize and load the Shadow WASM module
   * @returns Promise that resolves to the initialized WASM module
   *
   * @example
   * ```typescript
   * await Shadow.init();
   * const shadow = Shadow.create('contract Foo {}');
   * ```
   */
  static async init(): Promise<MainModule> {
    if (wasmModule) return wasmModule;
    if (initPromise) return initPromise;

    const promise = createShadowModule().then((module) => {
      wasmModule = module;
      return module;
    });

    initPromise = promise;
    return promise;
  }

  /**
   * Parse Solidity source code and return AST as JSON
   *
   * @param source - Solidity source code
   * @param name - Optional source file name (defaults to empty string)
   * @returns JSON string containing the AST
   *
   * @example
   * ```typescript
   * await Shadow.init();
   * const ast = Shadow.parseSource('contract Foo {}', 'Foo.sol');
   * const parsed = JSON.parse(ast);
   * ```
   */
  static parseSource(source: string, name: string = ''): SourceUnit {
    if (!wasmModule) throw new Error('Shadow module not loaded. Call await Shadow.init() first.');
    const ast = wasmModule.Shadow.parseSource(source, name);
    return JSON.parse(ast);
  }

  /**
   * Create a Shadow instance from source code
   *
   * @param source - Shadow contract source code
   * @returns Shadow instance
   *
   * @example
   * ```typescript
   * await Shadow.init();
   * const shadow = Shadow.create(`
   *   contract ShadowFoo {
   *     function bar() public {}
   *   }
   * `);
   * ```
   */
  static create(source: string): Shadow {
    if (!wasmModule) throw new Error('Shadow module not loaded. Call await Shadow.init() first.');
    const native = new wasmModule.Shadow(source);
    return new Shadow(native);
  }


  /**
   * Stitch shadow contract into target source code
   *
   * @param target - Target Solidity source code
   * @param sourceName - Optional source name for the result
   * @param contractName - Optional specific contract name to target
   * @returns Modified source code with shadow contract stitched in
   *
   * @example
   * ```typescript
   * const result = shadow.stitchIntoSource(
   *   'contract Target {}',
   *   'Target.sol',
   *   'Target'
   * );
   * ```
   */
  stitchIntoSource(
    target: string,
    sourceName: string = '',
    contractName: string = ''
  ): SourceUnit {
    if (!this.native) throw new Error('Shadow instance has been destroyed');
    const ast = this.native.stitchIntoSource(target, sourceName, contractName);
    return JSON.parse(ast);
  }

  /**
   * Stitch shadow contract into target AST (JSON)
   *
   * @param targetAst - Target AST as JSON string
   * @param contractName - Optional specific contract name to target
   * @returns Modified AST as JSON string with shadow contract stitched in
   *
   * @example
   * ```typescript
   * const targetAst = await parseSource('contract Target {}');
   * const result = shadow.stitchIntoAst(targetAst, 'Target');
   * const modifiedAst = JSON.parse(result);
   * ```
   */
  stitchIntoAst(targetAst: string, contractName: string = ''): SourceUnit {
    if (!this.native) throw new Error('Shadow instance has been destroyed');
    const ast = this.native.stitchIntoAst(targetAst, contractName);
    return JSON.parse(ast);
  }

  /**
   * Destroy the native Shadow instance and free WASM memory
   *
   * Call this when you're done with the Shadow instance to prevent memory leaks.
   * After calling destroy(), the instance can no longer be used.
   *
   * @example
   * ```typescript
   * await Shadow.init();
   * const shadow = Shadow.create('contract Foo {}');
   * // ... use shadow ...
   * shadow.destroy();
   * ```
   */
  destroy(): void {
    if (this.native) {
      this.native.delete();
      this.native = null;
    }
  }

  /**
   * Implement Symbol.dispose for explicit resource management (when available)
   *
   * Note: Requires TypeScript 5.2+ and target lib 'esnext'
   *
   * @example
   * ```typescript
   * await Shadow.init();
   * {
   *   using shadow = Shadow.create('contract Foo {}');
   *   shadow.stitchIntoSource('contract Bar {}');
   *   // Automatically destroyed at end of block
   * }
   * ```
   */
  // Commented out to maintain compatibility - uncomment if using TS 5.2+ with esnext
  // [Symbol.dispose](): void {
  //   this.destroy();
  // }
}
