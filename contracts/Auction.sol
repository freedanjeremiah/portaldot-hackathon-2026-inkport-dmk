// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Auction {
    address public beneficiary;
    uint256 public auctionEnd;
    address public highestBidder;
    uint256 public highestBid;

    event HighestBidIncreased(address indexed bidder, uint256 amount);

    constructor(uint256 biddingTime) {
        beneficiary = msg.sender;
        auctionEnd = block.timestamp + biddingTime;
        highestBid = 0;
    }

    function bid() public payable {
        require(block.timestamp <= auctionEnd);
        require(msg.value > highestBid);
        highestBidder = msg.sender;
        highestBid = msg.value;
        emit HighestBidIncreased(msg.sender, msg.value);
    }

    function highestBidOf() public view returns (uint256) {
        return highestBid;
    }
}
