// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Flipper {
    bool value;

    constructor(bool initial) {
        value = initial;
    }

    function flip() public {
        value = !value;
    }

    function get() public view returns (bool) {
        return value;
    }
}
