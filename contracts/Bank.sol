// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Bank {
    mapping(address => uint256) deposits;

    function deposit() public payable {
        deposits[msg.sender] = deposits[msg.sender] + msg.value;
    }

    function balanceOf(address who) public view returns (uint256) {
        return deposits[who];
    }

    function withdraw(uint256 amount) public {
        require(deposits[msg.sender] >= amount);
        deposits[msg.sender] = deposits[msg.sender] - amount;
        payable(msg.sender).transfer(amount);
    }
}
