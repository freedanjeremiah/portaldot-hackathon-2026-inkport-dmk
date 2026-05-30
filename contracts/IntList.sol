// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract IntList {
    uint256[] items;

    function add(uint256 x) public {
        items.push(x);
    }

    function len() public view returns (uint256) {
        return items.length;
    }

    function get(uint256 i) public view returns (uint256) {
        return items[i];
    }

    function set(uint256 i, uint256 x) public {
        items[i] = x;
    }

    function sum() public view returns (uint256) {
        uint256 s = 0;
        for (uint256 i = 0; i < items.length; i++) {
            s += items[i];
        }
        return s;
    }
}
