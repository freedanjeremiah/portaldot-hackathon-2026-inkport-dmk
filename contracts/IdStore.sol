// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract IdStore {
    mapping(uint256 => uint256) byId;
    function set(uint256 id, uint256 v) public { byId[id] = v; }
    function get(uint256 id) public view returns (uint256) { return byId[id]; }
}
