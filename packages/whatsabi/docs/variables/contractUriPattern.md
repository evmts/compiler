[**@tevm/whatsabi**](../README.md)

***

[@tevm/whatsabi](../globals.md) / contractUriPattern

# Variable: contractUriPattern

> `const` **contractUriPattern**: `RegExp`

Defined in: [packages/whatsabi/src/contractUriPattern.js:10](https://github.com/evmts/compiler/blob/main/packages/whatsabi/src/contractUriPattern.js#L10)

Regular expression pattern for matching contract URIs.
Looks like evm://<chainId>/<address>?<query>
Valid query params (all optional)
- rpcUrl: string
- etherscanBaseUrl: string
- followProxies: boolean
- etherscanApiKey: string
