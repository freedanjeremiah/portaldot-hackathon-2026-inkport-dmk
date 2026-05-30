// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Narrow16 {
    uint16 x;

    constructor() {
        x = 65000;
    }

    function addc(uint16 n) public {
        x = x + n;
    }

    function get() public view returns (uint16) {
        return x;
    }
}
