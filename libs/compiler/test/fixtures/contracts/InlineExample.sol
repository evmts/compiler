// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract InlineExample {
  uint256 private stored;

  function set(uint256 newValue) external {
    stored = newValue;
  }

  function get() external view returns (uint256) {
    return stored;
  }
}
