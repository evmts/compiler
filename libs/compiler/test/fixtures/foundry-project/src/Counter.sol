// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

contract Counter {
    uint256 private _value;

    function increment() external {
        _value += 1;
    }

    function current() external view returns (uint256) {
        return _value;
    }
}
