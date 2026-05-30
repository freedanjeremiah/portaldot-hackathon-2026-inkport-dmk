// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MinMax {
    function minmax(uint a, uint b) public pure returns (uint, uint) {
        if (a < b) { return (a, b); }
        return (b, a);
    }
}
