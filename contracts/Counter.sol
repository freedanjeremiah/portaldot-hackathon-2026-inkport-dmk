// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Counter {
    uint256 count;

    constructor(uint256 initial) {
        count = initial;
    }

    function inc() public {
        count = count + 1;
    }

    function incBy(uint256 n) public {
        count = count + n;
    }

    function get() public view returns (uint256) {
        return count;
    }
}
