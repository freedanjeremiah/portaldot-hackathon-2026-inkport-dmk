// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Ownable {
    address owner;
    uint256 storedValue;

    modifier onlyOwner() {
        require(msg.sender == owner);
        _;
    }

    constructor() {
        owner = msg.sender;
    }

    function owner() public view returns (address) {
        return owner;
    }

    function transferOwnership(address newOwner) public onlyOwner {
        owner = newOwner;
    }

    function setValue(uint256 v) public onlyOwner {
        storedValue = v;
    }

    function value() public view returns (uint256) {
        return storedValue;
    }
}
