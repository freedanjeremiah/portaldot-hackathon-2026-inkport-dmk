// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Bits {
    function mask(uint x) public pure returns (uint) { return x & 0xff; }
    function shl(uint x) public pure returns (uint) { return x << 2; }
}
