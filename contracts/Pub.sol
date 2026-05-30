// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Pub {
    uint256 public count;
    constructor(uint256 c) { count = c; }
    function bump() public { count = count + 1; }
}
