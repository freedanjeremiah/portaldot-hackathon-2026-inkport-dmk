// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Signed {
    int256 x;
    constructor(int256 v) { x = v; }
    function dec() public { x = x - 1; }
    function add(int256 d) public { x = x + d; }
    function get() public view returns (int256) { return x; }
}
