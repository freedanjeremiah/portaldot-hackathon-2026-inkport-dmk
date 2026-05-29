// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract ERC20 {
    mapping(address => uint256) balances;
    mapping(address => mapping(address => uint256)) allowances;
    uint256 totalSupply;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    constructor(uint256 supply) {
        totalSupply = supply;
        balances[msg.sender] = supply;
    }

    function totalSupply() public view returns (uint256) {
        return totalSupply;
    }

    function balanceOf(address who) public view returns (uint256) {
        return balances[who];
    }

    function allowance(address owner, address spender) public view returns (uint256) {
        return allowances[owner][spender];
    }

    function transfer(address to, uint256 value) public returns (bool) {
        require(balances[msg.sender] >= value);
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

    function transferFrom(address from, address to, uint256 value) public returns (bool) {
        require(balances[from] >= value);
        require(allowances[from][msg.sender] >= value);
        balances[from] = balances[from] - value;
        balances[to] = balances[to] + value;
        allowances[from][msg.sender] = allowances[from][msg.sender] - value;
        emit Transfer(from, to, value);
        return true;
    }
}
