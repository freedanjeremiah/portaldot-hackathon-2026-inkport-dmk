// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Overload {
    uint256 s;

    function add(uint256 a) public {
        s += a;
    }

    function add(uint256 a, uint256 b) public {
        s += a + b;
    }

    function get() public view returns (uint256) {
        return s;
    }
}
