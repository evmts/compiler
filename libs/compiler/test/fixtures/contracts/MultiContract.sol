// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract First {
  uint256 public exposedValue;
}

contract Second {
  string internal name;
}

contract Target {
  uint256 private secret;

  function internalOnly() internal view returns (uint256) {
    return secret;
  }

  function externalOnly() external view returns (uint256) {
    return secret;
  }
}
