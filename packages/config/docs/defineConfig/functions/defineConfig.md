[**@tevm/config**](../../README.md)

***

[@tevm/config](../../modules.md) / [defineConfig](../README.md) / defineConfig

# Function: defineConfig()

> **defineConfig**(`configFactory`): `object`

Defined in: [packages/config/src/defineConfig.js:48](https://github.com/evmts/compiler/blob/main/packages/config/src/defineConfig.js#L48)

Typesafe way to create an Tevm CompilerConfig

## Parameters

### configFactory

[`ConfigFactory`](../../types/type-aliases/ConfigFactory.md)

## Returns

`object`

### configFn()

> **configFn**: (`configFilePath`) => `Effect`\<[`ResolvedCompilerConfig`](../../types/type-aliases/ResolvedCompilerConfig.md), [`DefineConfigError`](../classes/DefineConfigError.md), `never`\>

#### Parameters

##### configFilePath

`string`

#### Returns

`Effect`\<[`ResolvedCompilerConfig`](../../types/type-aliases/ResolvedCompilerConfig.md), [`DefineConfigError`](../classes/DefineConfigError.md), `never`\>

## Example

```ts
import { defineConfig } from '@tevm/ts-plugin'

export default defineConfig(() => ({
	lib: ['lib'],
	remappings: {
	  'foo': 'foo/bar'
	}
})
```
