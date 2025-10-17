// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract WarningContract {
  function shouldWarn(uint256 value) public {
    value + 1;
  }
}
