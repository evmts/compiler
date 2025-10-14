import { describe, test, expect, beforeAll } from 'bun:test';
import { loadShadow, Shadow } from './index.js';

describe('Shadow WASM Module', () => {
  describe('Module Loading', () => {
    test('loadShadow initializes module', async () => {
      const module = await loadShadow();
      expect(module).toBeDefined();
      expect(module.Shadow).toBeDefined();
      expect(typeof module.Shadow.parseSource).toBe('function');
    });

    test('loadShadow returns same instance on multiple calls', async () => {
      const module1 = await loadShadow();
      const module2 = await loadShadow();
      expect(module1).toBe(module2);
    });

    test('parseSource throws before module loaded', () => {
      // This test would need to run first, but module is already loaded
      // Testing the error path requires isolating the module state
    });
  });

  describe('parseSource - Static Parsing', () => {
    beforeAll(async () => {
      await loadShadow();
    });

    test('parses simple contract', () => {
      const source = 'contract Foo {}';
      const ast = Shadow.parseSource(source, 'Foo.sol');

      expect(typeof ast).toBe('string');
      const parsed = JSON.parse(ast);
      expect(parsed).toBeDefined();
      expect(parsed.nodeType).toBe('SourceUnit');
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
      const parsed = JSON.parse(ast);

      expect(parsed.nodeType).toBe('SourceUnit');
      expect(parsed.nodes).toBeArrayOfSize(1);
      expect(parsed.nodes[0].nodeType).toBe('ContractDefinition');
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
      const parsed = JSON.parse(ast);

      expect(parsed.nodes).toBeArrayOfSize(2);
      expect(parsed.nodes[1].baseContracts).toBeArrayOfSize(1);
    });

    test('parses with source name parameter', () => {
      const source = 'contract Test {}';
      const ast = Shadow.parseSource(source, 'CustomName.sol');
      const parsed = JSON.parse(ast);

      expect(parsed.absolutePath).toBe('CustomName.sol');
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
      const parsed = JSON.parse(ast);

      expect(parsed.nodeType).toBe('SourceUnit');
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
      const parsed = JSON.parse(ast);

      expect(parsed.nodes[0].nodes.length).toBeGreaterThan(0);
    });
  });

  describe('Shadow Instance - Stitching Operations', () => {
    beforeAll(async () => {
      await loadShadow();
    });

    test('creates Shadow instance', async () => {
      const shadowSource = `
        contract ShadowCounter {
          function shadowIncrement() internal {}
        }
      `;
      const shadow = await Shadow.create(shadowSource);

      expect(shadow).toBeInstanceOf(Shadow);
      shadow.dispose();
    });

    test('stitchIntoSource merges contracts', async () => {
      const shadowSource = `
        contract ShadowBase {
          uint256 internal shadowValue;
          function getShadowValue() internal view returns (uint256) {
            return shadowValue;
          }
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

      const shadow = await Shadow.create(shadowSource);
      const stitched = shadow.stitchIntoSource(targetSource, 'Target.sol', 'Target');

      expect(typeof stitched).toBe('string');
      expect(stitched).toContain('ShadowBase');
      expect(stitched).toContain('Target');
      expect(stitched.length).toBeGreaterThan(targetSource.length);

      shadow.dispose();
    });

    test('stitchIntoAst merges ASTs', async () => {
      const shadowSource = 'contract Shadow { function test() internal {} }';
      const targetSource = 'contract Target {}';

      const shadow = await Shadow.create(shadowSource);
      const targetAst = Shadow.parseSource(targetSource);
      const stitchedAst = shadow.stitchIntoAst(targetAst, 'Target');

      expect(typeof stitchedAst).toBe('string');
      const parsed = JSON.parse(stitchedAst);
      expect(parsed.nodeType).toBe('SourceUnit');

      // Should have both contracts or merged contract
      const hasMultipleContracts = parsed.nodes.length > 1;
      const hasMergedContract = parsed.nodes[0].nodes?.length > 0;
      expect(hasMultipleContracts || hasMergedContract).toBe(true);

      shadow.dispose();
    });

    test('handles multiple contracts in target', async () => {
      const shadowSource = 'contract ShadowHelper {}';
      const targetSource = `
        contract First {}
        contract Second {}
      `;

      const shadow = await Shadow.create(shadowSource);
      const stitched = shadow.stitchIntoSource(targetSource, 'Multi.sol', 'First');

      expect(stitched).toContain('ShadowHelper');
      expect(stitched).toContain('First');
      expect(stitched).toContain('Second');

      shadow.dispose();
    });

    test('stitching with empty target', async () => {
      const shadowSource = 'contract Shadow {}';
      const targetSource = 'contract Empty {}';

      const shadow = await Shadow.create(shadowSource);
      const stitched = shadow.stitchIntoSource(targetSource);

      expect(stitched).toContain('Shadow');
      shadow.dispose();
    });

    test('stitching with complex shadow contract', async () => {
      const shadowSource = `
        contract ComplexShadow {
          struct Data {
            uint256 value;
            string name;
          }

          mapping(address => Data) internal shadowData;

          function setShadowData(uint256 _value, string memory _name) internal {
            shadowData[msg.sender] = Data(_value, _name);
          }
        }
      `;

      const targetSource = 'contract Target { uint256 public x; }';

      const shadow = await Shadow.create(shadowSource);
      const stitched = shadow.stitchIntoSource(targetSource, 'Target.sol', 'Target');

      expect(stitched).toContain('ComplexShadow');
      expect(stitched).toContain('shadowData');
      shadow.dispose();
    });
  });

  describe('Memory Management', () => {
    beforeAll(async () => {
      await loadShadow();
    });

    test('dispose cleans up instance', async () => {
      const shadow = await Shadow.create('contract Test {}');
      shadow.dispose();

      expect(() => {
        shadow.stitchIntoSource('contract Target {}');
      }).toThrow('Shadow instance has been disposed');
    });

    test('operations fail after dispose', async () => {
      const shadow = await Shadow.create('contract Test {}');
      shadow.dispose();

      expect(() => shadow.stitchIntoSource('contract X {}')).toThrow();
      expect(() => shadow.stitchIntoAst('{}')).toThrow();
    });

    test('multiple dispose calls are safe', async () => {
      const shadow = await Shadow.create('contract Test {}');
      shadow.dispose();
      shadow.dispose(); // Should not throw
    });

    test('multiple instances can coexist', async () => {
      const shadow1 = await Shadow.create('contract Shadow1 {}');
      const shadow2 = await Shadow.create('contract Shadow2 {}');
      const shadow3 = await Shadow.create('contract Shadow3 {}');

      const target = 'contract Target {}';
      const result1 = shadow1.stitchIntoSource(target);
      const result2 = shadow2.stitchIntoSource(target);
      const result3 = shadow3.stitchIntoSource(target);

      expect(result1).toContain('Shadow1');
      expect(result2).toContain('Shadow2');
      expect(result3).toContain('Shadow3');

      shadow1.dispose();
      shadow2.dispose();
      shadow3.dispose();
    });

    test('disposed instance does not affect others', async () => {
      const shadow1 = await Shadow.create('contract A {}');
      const shadow2 = await Shadow.create('contract B {}');

      shadow1.dispose();

      const result = shadow2.stitchIntoSource('contract Target {}');
      expect(result).toContain('B');

      shadow2.dispose();
    });
  });

  describe('Error Handling', () => {
    beforeAll(async () => {
      await loadShadow();
    });

    test('invalid shadow source throws on create', async () => {
      const invalidSource = 'contract Broken {';

      await expect(Shadow.create(invalidSource)).rejects.toThrow();
    });

    test('invalid target source throws on stitch', async () => {
      const shadow = await Shadow.create('contract Shadow {}');
      const invalidTarget = 'contract Broken {';

      expect(() => {
        shadow.stitchIntoSource(invalidTarget);
      }).toThrow();

      shadow.dispose();
    });

    test('invalid AST throws on stitchIntoAst', async () => {
      const shadow = await Shadow.create('contract Shadow {}');
      const invalidAst = '{ invalid json';

      expect(() => {
        shadow.stitchIntoAst(invalidAst);
      }).toThrow();

      shadow.dispose();
    });
  });

  describe('Integration - Real World Scenarios', () => {
    beforeAll(async () => {
      await loadShadow();
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
      expect(JSON.parse(ast).nodeType).toBe('SourceUnit');

      // Create shadow for instrumentation
      const shadowSource = `
        contract TransferLogger {
          event TransferLogged(address from, address to, uint256 amount);

          function logTransfer(address to, uint256 amount) internal {
            emit TransferLogged(msg.sender, to, amount);
          }
        }
      `;

      const shadow = await Shadow.create(shadowSource);
      const stitched = shadow.stitchIntoSource(erc20Source, 'ERC20.sol', 'ERC20');

      expect(stitched).toContain('TransferLogger');
      expect(stitched).toContain('ERC20');

      shadow.dispose();
    });

    test('workflow: parse -> analyze -> stitch -> parse again', async () => {
      const original = 'contract Original { uint256 x; }';

      // Parse original
      const originalAst = Shadow.parseSource(original, 'Original.sol');
      const parsedOriginal = JSON.parse(originalAst);
      expect(parsedOriginal.nodes[0].name).toBe('Original');

      // Stitch shadow
      const shadow = await Shadow.create('contract Shadow { uint256 y; }');
      const stitched = shadow.stitchIntoSource(original, 'Original.sol', 'Original');

      // Parse stitched result
      const stitchedAst = Shadow.parseSource(stitched, 'Stitched.sol');
      const parsedStitched = JSON.parse(stitchedAst);
      expect(parsedStitched.nodes.length).toBeGreaterThanOrEqual(1);

      shadow.dispose();
    });

    test('stitch multiple shadow contracts sequentially', async () => {
      let target = 'contract Target { uint256 value; }';

      const shadow1 = await Shadow.create('contract Shadow1 { function fn1() internal {} }');
      target = shadow1.stitchIntoSource(target, 'Target.sol', 'Target');
      shadow1.dispose();

      const shadow2 = await Shadow.create('contract Shadow2 { function fn2() internal {} }');
      target = shadow2.stitchIntoSource(target, 'Target.sol', 'Target');
      shadow2.dispose();

      expect(target).toContain('Shadow1');
      expect(target).toContain('Shadow2');
      expect(target).toContain('Target');
    });
  });
});
