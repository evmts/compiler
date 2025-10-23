// SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.30;

/// @notice Minimal hook interface matching the Uniswap v4 PoolManager expectations.
interface IHooks {
	function beforeSwap(address sender, bytes calldata data) external returns (bytes memory);
	function afterSwap(address sender, bytes calldata data) external returns (bytes memory);
}

/// @notice Simplified PoolKey capturing the hook address like the canonical contract.
struct PoolKey {
	address currency0;
	address currency1;
	uint24 fee;
	int24 tickSpacing;
	address hooks;
}

/// @notice Minimal PoolManager surface used for integration shadowing tests.
contract PoolManagerMock {
	mapping(bytes32 => PoolKey) private pools;
	address public lastHookInvoked;
	bytes32 public lastRegisteredPoolId;

	function registerPool(PoolKey calldata key) external returns (bytes32 poolId) {
		poolId = keccak256(abi.encode(key));
		pools[poolId] = key;
		lastRegisteredPoolId = poolId;
	}

	function poolHooks(bytes32 poolId) public view returns (address) {
		return pools[poolId].hooks;
	}

	function dispatchSwap(bytes32 poolId, bytes calldata data) external returns (bytes memory result) {
		address hook = pools[poolId].hooks;
		if (hook == address(0)) return '';
		lastHookInvoked = hook;
		return IHooks(hook).beforeSwap(msg.sender, data);
	}
}

/// @notice Hook implementation that records the last sender and calldata.
contract LoggingHooksMock is IHooks {
	address public lastSender;
	bytes public lastData;

	function beforeSwap(address sender, bytes calldata data) external override returns (bytes memory) {
		lastSender = sender;
		lastData = data;
		return data;
	}

	function afterSwap(address sender, bytes calldata data) external override returns (bytes memory) {
		lastSender = sender;
		lastData = data;
		return data;
	}
}
