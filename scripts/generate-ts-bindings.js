#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

const WASM_PATH = path.join(__dirname, '../zig-out/lib/libshadow-wasm.a');
const OUTPUT_DIR = path.join(__dirname, '../dist');
const TS_FILE = path.join(OUTPUT_DIR, 'shadow.ts');
const DTS_FILE = path.join(OUTPUT_DIR, 'shadow.d.ts');

// Create output directory
if (!fs.existsSync(OUTPUT_DIR)) {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
}

console.log('Generating TypeScript bindings for Shadow WASM module...');

// TypeScript implementation
const tsContent = `/**
 * Shadow - Pure Syntax Parser for Solidity
 *
 * This module provides a WASM-based interface to the Solidity parser
 * that can parse Solidity code without requiring semantic validity.
 */

let wasmModule: WebAssembly.Instance | null = null;
let wasmMemory: WebAssembly.Memory | null = null;

export interface ParseResult {
  ast: any;
  errors?: string[];
}

export class Shadow {
  private ctx: number = 0;

  constructor() {
    if (!wasmModule) {
      throw new Error('WASM module not initialized. Call Shadow.init() first.');
    }
  }

  /**
   * Initialize the WASM module
   * Must be called before creating Shadow instances
   */
  static async init(wasmPath: string): Promise<void> {
    const wasmBuffer = await fetch(wasmPath).then(r => r.arrayBuffer());
    const wasmImports = {
      env: {
        memory: new WebAssembly.Memory({ initial: 256, maximum: 512 }),
      },
    };

    const { instance } = await WebAssembly.instantiate(wasmBuffer, wasmImports);
    wasmModule = instance;
    wasmMemory = wasmImports.env.memory;
  }

  /**
   * Parse a Solidity function into an AST
   *
   * @param functionSource - Function source code (just the function, no contract wrapper)
   * @returns Parsed AST as JSON
   *
   * @example
   * \`\`\`typescript
   * const shadow = new Shadow();
   * const ast = shadow.parseFunction(\`
   *   function test() public {
   *     return undefinedVariable + 5;
   *   }
   * \`);
   * console.log(ast);
   * \`\`\`
   */
  parseFunction(functionSource: string): ParseResult {
    if (!wasmModule || !wasmModule.exports) {
      throw new Error('WASM module not initialized');
    }

    // Call WASM function to parse
    // Note: Actual implementation depends on exported functions from Zig
    const parseFunc = wasmModule.exports.sol_parser_parse as Function;

    // Convert string to WASM memory
    const encoder = new TextEncoder();
    const bytes = encoder.encode(functionSource);

    // Allocate memory in WASM
    const allocFunc = wasmModule.exports.allocate as Function;
    const ptr = allocFunc(bytes.length);

    // Copy string to WASM memory
    const memory = new Uint8Array((wasmMemory as WebAssembly.Memory).buffer);
    memory.set(bytes, ptr);

    // Call parser
    const resultPtr = parseFunc(this.ctx, ptr, bytes.length);

    // Read result from WASM memory
    const decoder = new TextDecoder();
    const resultBytes = new Uint8Array((wasmMemory as WebAssembly.Memory).buffer, resultPtr);
    const resultStr = decoder.decode(resultBytes);

    // Free WASM memory
    const freeFunc = wasmModule.exports.free as Function;
    freeFunc(ptr);
    freeFunc(resultPtr);

    return {
      ast: JSON.parse(resultStr),
    };
  }

  /**
   * Parse a full Solidity contract
   *
   * @param contractSource - Complete contract source code
   * @returns Parsed AST as JSON
   */
  parseContract(contractSource: string): ParseResult {
    // Similar implementation to parseFunction
    // but for full contracts
    throw new Error('Not yet implemented');
  }

  /**
   * Stitch a shadow function into a valid contract's AST
   *
   * @param originalAst - AST of the original contract
   * @param shadowAst - AST of the shadow function
   * @returns Combined AST
   */
  static stitchAST(originalAst: any, shadowAst: any): any {
    // Navigate to contract node
    const contract = originalAst.nodes.find((n: any) => n.nodeType === 'ContractDefinition');
    if (!contract) {
      throw new Error('No contract found in original AST');
    }

    // Extract shadow function
    const shadowContract = shadowAst.nodes.find((n: any) => n.nodeType === 'ContractDefinition');
    if (!shadowContract || !shadowContract.nodes || shadowContract.nodes.length === 0) {
      throw new Error('No function found in shadow AST');
    }

    const shadowFunction = shadowContract.nodes[0];

    // Add shadow function to original contract
    contract.nodes.push(shadowFunction);

    return originalAst;
  }

  /**
   * Clean up resources
   */
  destroy(): void {
    // Clean up WASM resources if needed
  }
}

export default Shadow;
`;

// TypeScript type definitions
const dtsContent = `/**
 * Shadow - Pure Syntax Parser for Solidity
 * Type definitions
 */

export interface ParseResult {
  ast: any;
  errors?: string[];
}

export declare class Shadow {
  constructor();

  /**
   * Initialize the WASM module
   * Must be called before creating Shadow instances
   */
  static init(wasmPath: string): Promise<void>;

  /**
   * Parse a Solidity function into an AST
   */
  parseFunction(functionSource: string): ParseResult;

  /**
   * Parse a full Solidity contract
   */
  parseContract(contractSource: string): ParseResult;

  /**
   * Stitch a shadow function into a valid contract's AST
   */
  static stitchAST(originalAst: any, shadowAst: any): any;

  /**
   * Clean up resources
   */
  destroy(): void;
}

export default Shadow;
`;

// Write files
fs.writeFileSync(TS_FILE, tsContent);
fs.writeFileSync(DTS_FILE, dtsContent);

console.log(`✓ Generated ${TS_FILE}`);
console.log(`✓ Generated ${DTS_FILE}`);
console.log('\nTypeScript bindings generated successfully!');
console.log('Import with: import Shadow from "./dist/shadow";');
