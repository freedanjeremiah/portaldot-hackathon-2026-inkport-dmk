// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Escrow {
    address public buyer;
    address public seller;
    address public arbiter;
    uint256 public balance;
    bool public released;

    modifier onlyArbiter() {
        require(msg.sender == arbiter);
        _;
    }

    constructor(address _seller, address _arbiter) {
        buyer = msg.sender;
        seller = _seller;
        arbiter = _arbiter;
        released = false;
    }

    function deposit() public payable {
        balance = balance + msg.value;
    }

    function release() public onlyArbiter {
        require(!released);
        released = true;
        payable(seller).transfer(balance);
        balance = 0;
    }
}
