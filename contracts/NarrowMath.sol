// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract NarrowMath {
    uint8 x;

    constructor() {
        x = 250;
    }

    function addc(uint8 n) public {
        x = x + n;
    }

    function get() public view returns (uint8) {
        return x;
    }
}
