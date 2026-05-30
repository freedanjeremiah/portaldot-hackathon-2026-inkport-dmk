// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract ERC721 {
    mapping(uint256 => address) owners;
    mapping(address => uint256) balances;
    mapping(uint256 => address) tokenApprovals;
    mapping(address => mapping(address => bool)) operatorApprovals;

    event Transfer(address indexed from, address indexed to, uint256 indexed id);
    event Approval(address indexed owner, address indexed approved, uint256 indexed id);
    event ApprovalForAll(address indexed owner, address indexed operator, bool approved);

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

    function approve(address to, uint256 id) public {
        require(owners[id] == msg.sender);
        tokenApprovals[id] = to;
        emit Approval(msg.sender, to, id);
    }

    function getApproved(uint256 id) public view returns (address) {
        return tokenApprovals[id];
    }

    function setApprovalForAll(address operator, bool approved) public {
        operatorApprovals[msg.sender][operator] = approved;
        emit ApprovalForAll(msg.sender, operator, approved);
    }

    function isApprovedForAll(address owner, address operator) public view returns (bool) {
        return operatorApprovals[owner][operator];
    }

    function transferFrom(address from, address to, uint256 id) public {
        require(owners[id] == from);
        require(to != address(0));
        require(
            msg.sender == from ||
            tokenApprovals[id] == msg.sender ||
            operatorApprovals[from][msg.sender]
        );
        owners[id] = to;
        tokenApprovals[id] = address(0);
        balances[from] = balances[from] - 1;
        balances[to] = balances[to] + 1;
        emit Transfer(from, to, id);
    }
}
