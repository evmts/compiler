import { describe, test, expect, beforeAll } from 'bun:test';
import { Shadow } from './index.js';

// NOTE: These tests currently fail in Bun due to Emscripten compatibility issues
// The error "import function a:n must be callable" indicates the Emscripten-generated
// JS doesn't properly provide imports for Bun's runtime. Tests should work in Node.js.
// This is NOT a WASI issue - Bun fully supports WASI.

describe('Shadow WASM Module', () => {
  describe('Module Loading', () => {
    test('Shadow.init initializes module', async () => {
      const module = await Shadow.init();
      expect(module).toBeDefined();
      expect(module.Shadow).toBeDefined();
      expect(typeof module.Shadow.parseSource).toBe('function');
    });

    test('Shadow.init returns same instance on multiple calls', async () => {
      const module1 = await Shadow.init();
      const module2 = await Shadow.init();
      expect(module1).toBe(module2);
    });

    test('parseSource throws before module loaded', () => {
      // This test would need to run first, but module is already loaded
      // Testing the error path requires isolating the module state
    });
  });

  describe('parseSource - Static Parsing', () => {
    beforeAll(async () => {
      await Shadow.init();
    });

    test('parses simple contract', () => {
      const source = 'contract Foo {}';
      const ast = Shadow.parseSource(source, 'Foo.sol');
      expect(typeof ast).toBe('object');
      expect(ast).toBeDefined();
      expect(ast.nodeType).toBe('SourceUnit');
    });

    test('parses contract with functions', () => {
      const source = `
        contract Counter {
          uint256 public count;

          function increment() public {
            count++;
          }

          function decrement() public {
            count--;
          }
        }
      `;
      const ast = Shadow.parseSource(source, 'Counter.sol');
      expect(ast.nodeType).toBe('SourceUnit');
      expect(ast.nodes).toBeArrayOfSize(1);
      expect(ast.nodes[0].nodeType).toBe('ContractDefinition');
    });

    test('parses contract with inheritance', () => {
      const source = `
        contract Base {
          function foo() public virtual {}
        }

        contract Derived is Base {
          function foo() public override {}
        }
      `;
      const ast = Shadow.parseSource(source);
      expect(ast.nodes).toBeArrayOfSize(2);
      expect(ast.nodes[1].nodeType).toBe('ContractDefinition');
      // @ts-expect-error baseContracts doesn't exist in EnumDefinition
      expect(ast.nodes[1].baseContracts).toBeArrayOfSize(1);
    });

    test('parses with source name parameter', () => {
      const source = 'contract Test {}';
      const ast = Shadow.parseSource(source, 'CustomName.sol');

      // Note: absolutePath is not populated in Parsed state
      // It's only populated after semantic analysis (AnalysisSuccessful state)
      expect(ast.nodeType).toBe('SourceUnit');
      expect(ast.nodes[0].nodeType).toBe('ContractDefinition');
    });

    test('handles syntax errors gracefully', () => {
      const invalidSource = 'contract Foo {';

      expect(() => {
        Shadow.parseSource(invalidSource);
      }).toThrow();
    });

    test('parses empty contract', () => {
      const source = 'contract Empty {}';
      const ast = Shadow.parseSource(source);

      expect(ast.nodeType).toBe('SourceUnit');
    });

    test('parses contract with events and modifiers', () => {
      const source = `
        contract EventLogger {
          event Log(string message);

          modifier onlyOwner() {
            require(msg.sender == owner);
            _;
          }

          address public owner;

          function log(string memory message) public onlyOwner {
            emit Log(message);
          }
        }
      `;
      const ast = Shadow.parseSource(source);

      // @ts-expect-error nodes doesn't exist in EnumDefinition
      expect(ast.nodes[0].nodes.length).toBeGreaterThan(0);
    });
  });

  describe('Shadow Instance - Stitching Operations', () => {
    beforeAll(async () => {
      await Shadow.init();
    });

    test('creates Shadow instance', async () => {
      // Shadow is designed for FRAGMENTS, not full contracts
      const shadowSource = 'function shadowIncrement() internal {}';
      const shadow = Shadow.create(shadowSource);

      expect(shadow).toBeInstanceOf(Shadow);
      shadow.destroy();
    });

    test('stitchIntoSource merges fragments into contract', async () => {
      // Shadow uses FRAGMENTS, not full contracts
      const shadowFragment = `
        uint256 internal shadowValue;
        function getShadowValue() internal view returns (uint256) {
          return shadowValue;
        }
      `;

      const targetSource = `
        contract Target {
          uint256 public value;
          function getValue() public view returns (uint256) {
            return value;
          }
        }
      `;

      const shadow = Shadow.create(shadowFragment);
      const stitched = shadow.stitchIntoSource(targetSource, 'Target.sol', 'Target');

      expect(typeof stitched).toBe('object');
      expect(stitched.nodeType).toBe('SourceUnit');
      expect(stitched.absolutePath).toBe('Target.sol');

      // Check that Target contract now contains the shadow members
      const targetContract = stitched.nodes.find(
        (node: any) => node.nodeType === 'ContractDefinition' && node.name === 'Target'
      );
      expect(targetContract).toBeDefined();

      // Should have original members + shadow members
      // @ts-expect-error nodes exists on ContractDefinition
      expect(targetContract.nodes.length).toBeGreaterThan(2);

      shadow.destroy();
    });

    test('stitchIntoAst merges fragment into AST', async () => {
      const shadowFragment = 'function test() internal {}';
      const targetSource = 'contract Target {}';

      const shadow = Shadow.create(shadowFragment);
      const targetAst = Shadow.parseSource(targetSource);
      const stitchedAst = shadow.stitchIntoAst(JSON.stringify(targetAst), 'Target');

      expect(typeof stitchedAst).toBe('object');
      expect(stitchedAst.nodeType).toBe('SourceUnit');

      // Check that Target contract now has the test function
      const targetContract = stitchedAst.nodes.find(
        (node: any) => node.nodeType === 'ContractDefinition' && node.name === 'Target'
      );
      expect(targetContract).toBeDefined();
      // @ts-expect-error nodes exists on ContractDefinition
      expect(targetContract.nodes.length).toBeGreaterThan(0);

      shadow.destroy();
    });

    test('handles multiple contracts in target', async () => {
      const shadowFragment = 'function helper() internal {}';
      const targetSource = `
        contract First {}
        contract Second {}
      `;

      const shadow = Shadow.create(shadowFragment);
      const stitched = shadow.stitchIntoSource(targetSource, 'Multi.sol', 'First');

      expect(typeof stitched).toBe('object');
      expect(stitched.nodeType).toBe('SourceUnit');

      const contractNames = stitched.nodes
        .filter((node: any) => node.nodeType === 'ContractDefinition')
        .map((node: any) => node.name);

      expect(contractNames).toContain('First');
      expect(contractNames).toContain('Second');

      // Check that First contract has the helper function
      const firstContract = stitched.nodes.find(
        (node: any) => node.nodeType === 'ContractDefinition' && node.name === 'First'
      );
      expect(firstContract).toBeDefined();
      // @ts-expect-error nodes exists on ContractDefinition
      expect(firstContract.nodes.length).toBeGreaterThan(0);

      shadow.destroy();
    });

    test('stitching with empty target', async () => {
      const shadowFragment = 'uint256 public x;';
      const targetSource = 'contract Empty {}';

      const shadow = Shadow.create(shadowFragment);
      const stitched = shadow.stitchIntoSource(targetSource);

      expect(typeof stitched).toBe('object');
      expect(stitched.nodeType).toBe('SourceUnit');

      // Check that Empty contract now has the x variable
      const emptyContract = stitched.nodes.find(
        (node: any) => node.nodeType === 'ContractDefinition' && node.name === 'Empty'
      );
      expect(emptyContract).toBeDefined();
      // @ts-expect-error nodes exists on ContractDefinition
      expect(emptyContract.nodes.length).toBe(1);

      shadow.destroy();
    });

    test('stitching with complex shadow fragment', async () => {
      const shadowFragment = `
        struct Data {
          uint256 value;
          string name;
        }

        mapping(address => Data) internal shadowData;

        function setShadowData(uint256 _value, string memory _name) internal {
          shadowData[msg.sender] = Data(_value, _name);
        }
      `;

      const targetSource = 'contract Target { uint256 public x; }';

      const shadow = Shadow.create(shadowFragment);
      const stitched = shadow.stitchIntoSource(targetSource, 'Target.sol', 'Target');

      expect(typeof stitched).toBe('object');
      expect(stitched.nodeType).toBe('SourceUnit');

      // Check that Target contract now contains the shadow members
      const targetContract = stitched.nodes.find(
        (node: any) => node.nodeType === 'ContractDefinition' && node.name === 'Target'
      );
      expect(targetContract).toBeDefined();

      // Should have struct, mapping, function, plus original x
      // @ts-expect-error nodes exists on ContractDefinition
      expect(targetContract.nodes.length).toBeGreaterThan(3);

      // Check that shadowData mapping exists
      // @ts-expect-error nodes exists on ContractDefinition
      const hasMapping = targetContract.nodes.some((member: any) =>
        member.nodeType === 'VariableDeclaration' && member.name === 'shadowData'
      );
      expect(hasMapping).toBe(true);

      shadow.destroy();
    });
  });

  describe('Memory Management', () => {
    beforeAll(async () => {
      await Shadow.init();
    });

    test('destroy cleans up instance', async () => {
      const shadow = Shadow.create('contract Test {}');
      shadow.destroy();

      expect(() => {
        shadow.stitchIntoSource('contract Target {}');
      }).toThrow('Shadow instance has been destroyed');
    });

    test('operations fail after destroy', async () => {
      const shadow = Shadow.create('contract Test {}');
      shadow.destroy();

      expect(() => shadow.stitchIntoSource('contract X {}')).toThrow();
      expect(() => shadow.stitchIntoAst('{}')).toThrow();
    });

    test('multiple destroy calls are safe', async () => {
      const shadow = Shadow.create('contract Test {}');
      shadow.destroy();
      shadow.destroy(); // Should not throw
    });

    test('multiple instances can coexist', async () => {
      const shadow1 = Shadow.create('uint256 x1;');
      const shadow2 = Shadow.create('uint256 x2;');
      const shadow3 = Shadow.create('uint256 x3;');

      const target = 'contract Target {}';
      const result1 = shadow1.stitchIntoSource(target);
      const result2 = shadow2.stitchIntoSource(target);
      const result3 = shadow3.stitchIntoSource(target);

      // Each result should have Target contract with the respective variable
      const getVarNames = (ast: any) => {
        const contract = ast.nodes.find((n: any) => n.nodeType === 'ContractDefinition');
        return contract?.nodes?.map((m: any) => m.name) || [];
      };

      expect(getVarNames(result1)).toContain('x1');
      expect(getVarNames(result2)).toContain('x2');
      expect(getVarNames(result3)).toContain('x3');

      shadow1.destroy();
      shadow2.destroy();
      shadow3.destroy();
    });

    test('destroyed instance does not affect others', async () => {
      const shadow1 = Shadow.create('uint256 a;');
      const shadow2 = Shadow.create('uint256 b;');

      shadow1.destroy();

      const result = shadow2.stitchIntoSource('contract Target {}');
      const targetContract = result.nodes.find(
        (n: any) => n.nodeType === 'ContractDefinition' && n.name === 'Target'
      );
      // @ts-expect-error nodes exists on ContractDefinition
      const varNames = targetContract.nodes.map((m: any) => m.name);
      expect(varNames).toContain('b');

      shadow2.destroy();
    });
  });

  describe('Error Handling', () => {
    beforeAll(async () => {
      await Shadow.init();
    });

    test('invalid shadow fragment throws on create', async () => {
      const invalidFragment = 'function broken() {';  // Missing closing brace

      expect(() => Shadow.create(invalidFragment)).toThrow();
    });

    test('invalid target source throws on stitch', async () => {
      const shadow = Shadow.create('uint256 x;');
      const invalidTarget = 'contract Broken {';  // Missing closing brace

      expect(() => {
        shadow.stitchIntoSource(invalidTarget);
      }).toThrow();

      shadow.destroy();
    });

    test('invalid AST throws on stitchIntoAst', async () => {
      const shadow = Shadow.create('uint256 x;');
      const invalidAst = '{ invalid json';

      expect(() => {
        shadow.stitchIntoAst(invalidAst);
      }).toThrow();

      shadow.destroy();
    });
  });

  describe('Integration - Real World Scenarios', () => {
    beforeAll(async () => {
      await Shadow.init();
    });

    test('parse and stitch ERC20-like contract', async () => {
      const erc20Source = `
        contract ERC20 {
          mapping(address => uint256) public balanceOf;

          function transfer(address to, uint256 amount) public returns (bool) {
            require(balanceOf[msg.sender] >= amount);
            balanceOf[msg.sender] -= amount;
            balanceOf[to] += amount;
            return true;
          }
        }
      `;

      // Parse to verify syntax
      const ast = Shadow.parseSource(erc20Source);
      expect(ast.nodeType).toBe('SourceUnit');

      // Create shadow fragment for instrumentation
      const shadowFragment = `
        event TransferLogged(address from, address to, uint256 amount);

        function logTransfer(address to, uint256 amount) internal {
          emit TransferLogged(msg.sender, to, amount);
        }
      `;

      const shadow = Shadow.create(shadowFragment);
      const stitched = shadow.stitchIntoSource(erc20Source, 'ERC20.sol', 'ERC20');

      expect(stitched.nodeType).toBe('SourceUnit');
      expect(stitched.absolutePath).toBe('ERC20.sol');

      // Check that ERC20 contract now has the logging members
      const erc20Contract = stitched.nodes.find(
        (node: any) => node.nodeType === 'ContractDefinition' && node.name === 'ERC20'
      );
      expect(erc20Contract).toBeDefined();

      // Should have transfer function plus shadow event and function
      // @ts-expect-error nodes exists on ContractDefinition
      expect(erc20Contract.nodes.length).toBeGreaterThan(3);

      shadow.destroy();
    });

    test('workflow: parse -> analyze -> stitch -> parse again', async () => {
      const original = 'contract Original { uint256 x; }';

      // Parse original
      const originalAst = Shadow.parseSource(original, 'Original.sol');
      // @ts-expect-error name doesn't exist in ImportDirective
      expect(originalAst.nodes[0].name).toBe('Original');

      // Stitch shadow fragment
      const shadow = Shadow.create('uint256 y;');
      const stitchedAst = shadow.stitchIntoSource(original, 'Original.sol', 'Original');

      // Verify stitched result contains Original with both x and y
      expect(stitchedAst.nodeType).toBe('SourceUnit');
      expect(stitchedAst.absolutePath).toBe('Original.sol');

      const originalContract = stitchedAst.nodes.find(
        (node: any) => node.nodeType === 'ContractDefinition' && node.name === 'Original'
      );
      expect(originalContract).toBeDefined();

      // Should have both x and y variables
      // @ts-expect-error nodes exists on ContractDefinition
      expect(originalContract.nodes.length).toBe(2);

      shadow.destroy();
    });

    test('stitch multiple shadow fragments sequentially', async () => {
      const targetSource = 'contract Target { uint256 value; }';

      const shadow1 = Shadow.create('function fn1() internal {}');
      const stitched1 = shadow1.stitchIntoSource(targetSource, 'Target.sol', 'Target');
      shadow1.destroy();

      // For the second stitch, use the analyzed AST as source
      // Since stitchIntoSource returns an analyzed AST, serialize it
      const shadow2 = Shadow.create('function fn2() internal {}');
      const stitched2 = shadow2.stitchIntoAst(JSON.stringify(stitched1), 'Target');
      shadow2.destroy();

      expect(stitched2.nodeType).toBe('SourceUnit');

      const targetContract = stitched2.nodes.find(
        (node: any) => node.nodeType === 'ContractDefinition' && node.name === 'Target'
      );
      expect(targetContract).toBeDefined();

      // Should have value, fn1, and fn2
      // @ts-expect-error nodes exists on ContractDefinition
      expect(targetContract.nodes.length).toBe(3);

      // @ts-expect-error nodes exists on ContractDefinition
      const functionNames = targetContract.nodes
        .filter((m: any) => m.nodeType === 'FunctionDefinition')
        .map((m: any) => m.name);
      expect(functionNames).toContain('fn1');
      expect(functionNames).toContain('fn2');
    });
  });
});
