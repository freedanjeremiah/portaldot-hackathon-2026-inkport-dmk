// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Timed {
    uint256 start;
    constructor() { start = block.timestamp; }
    function elapsed() public view returns (uint256) { return block.timestamp - start; }
    function afterStart() public view returns (bool) { return block.timestamp >= start; }
}
