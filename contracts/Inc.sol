// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Inc {
    uint256 n;
    function bump() public { n++; }
    function addmul(uint a) public { n += a; }
    function get() public view returns (uint256) { return n; }
}
