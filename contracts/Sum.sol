// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Sum {
    function sumTo(uint n) public pure returns (uint) {
        uint s = 0;
        for (uint i = 0; i < n; i++) {
            s = s + i;
        }
        return s;
    }
}
