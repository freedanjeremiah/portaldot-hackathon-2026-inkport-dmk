// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract ERC721 {
    mapping(uint256 => address) owners;
    mapping(address => uint256) balances;

    event Transfer(address indexed from, address indexed to, uint256 indexed id);

    constructor() {}

    function mint(address to, uint256 id) public {
        require(owners[id] == address(0));
        owners[id] = to;
        balances[to] = balances[to] + 1;
        emit Transfer(address(0), to, id);
    }

    function ownerOf(uint256 id) public view returns (address) {
        return owners[id];
    }

    function balanceOf(address o) public view returns (uint256) {
        return balances[o];
    }

    function transferFrom(address from, address to, uint256 id) public {
        require(owners[id] == from);
        require(to != address(0));
        owners[id] = to;
        balances[from] = balances[from] - 1;
        balances[to] = balances[to] + 1;
        emit Transfer(from, to, id);
    }
}
