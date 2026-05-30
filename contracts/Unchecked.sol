// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Unchecked {
    function wrap(uint8 a) public pure returns (uint8) {
        unchecked {
            return a + 1;
        }
    }
}
