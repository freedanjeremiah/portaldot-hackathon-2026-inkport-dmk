// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Greeter {
    string greeting;

    constructor(string memory g) {
        greeting = g;
    }

    function setGreeting(string memory g) public {
        greeting = g;
    }

    function greet() public view returns (string memory) {
        return greeting;
    }
}
