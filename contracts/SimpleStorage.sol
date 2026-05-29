// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract SimpleStorage {
    uint256 data;

    constructor(uint256 initial) {
        data = initial;
    }

    function set(uint256 x) public {
        data = x;
    }

    function setIfPositive(uint256 x) public {
        require(x > 0);
        data = x;
    }

    function get() public view returns (uint256) {
        return data;
    }
}
