// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Token {
    mapping(address => uint256) private balances;
    mapping(address => mapping(address => uint256)) private allowances;
    uint256 private totalSupply_;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    error InsufficientBalance();
    error InsufficientAllowance();

    constructor(uint256 initialSupply) {
        totalSupply_ = initialSupply;
        balances[msg.sender] = initialSupply;
    }

    function balanceOf(address who) public view returns (uint256) {
        return balances[who];
    }

    function transfer(address to, uint256 value) public returns (bool) {
        if (balances[msg.sender] < value) { revert InsufficientBalance(); }
        balances[msg.sender] = balances[msg.sender] - value;
        balances[to] = balances[to] + value;
        emit Transfer(msg.sender, to, value);
        return true;
    }

    function approve(address spender, uint256 value) public returns (bool) {
        allowances[msg.sender][spender] = value;
        emit Approval(msg.sender, spender, value);
        return true;
    }
}
