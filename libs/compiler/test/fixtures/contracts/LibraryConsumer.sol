// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./MathLib.sol";

contract LibraryConsumer {
  function double(uint256 value) external pure returns (uint256) {
    return MathLib.double(value);
  }
}
