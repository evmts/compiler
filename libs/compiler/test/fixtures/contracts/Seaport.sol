// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

/// @notice Simplified structs mirroring Seaport order parameters.
struct OfferItem {
	address token;
	uint256 amount;
}

struct ConsiderationItem {
	address recipient;
	uint256 amount;
}

struct Order {
	address offerer;
	address fulfiller;
	OfferItem offer;
	ConsiderationItem[] consideration;
}

/// @notice Minimal marketplace that transfers balances for testing purposes.
contract SeaportMock {
	mapping(address => uint256) internal balances;

	event Transfer(address indexed from, address indexed to, uint256 amount);

	function seedBalance(address account, uint256 amount) external {
		balances[account] = amount;
	}

	function balanceOf(address account) external view returns (uint256) {
		return balances[account];
	}

	function fulfillOrder(Order memory order) public returns (bool fulfilled) {
		require(balances[order.offerer] >= order.offer.amount, 'insufficient offerer balance');
		balances[order.offerer] -= order.offer.amount;
		for (uint256 i = 0; i < order.consideration.length; i++) {
			ConsiderationItem memory item = order.consideration[i];
			balances[item.recipient] += item.amount;
			emit Transfer(order.offerer, item.recipient, item.amount);
		}
		balances[order.fulfiller] += order.offer.amount;
		emit Transfer(order.offerer, order.fulfiller, order.offer.amount);
		return true;
	}
}
