// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Voting {
    struct Proposal {
        uint256 votes;
    }

    mapping(uint256 => Proposal) proposals;
    uint256 public count;

    function addProposal() public {
        proposals[count].votes = 0;
        count++;
    }

    function vote(uint256 id) public {
        proposals[id].votes += 1;
    }

    function votesOf(uint256 id) public view returns (uint256) {
        return proposals[id].votes;
    }

    function winner() public view returns (uint256) {
        uint256 best = 0;
        uint256 bi = 0;
        for (uint256 i = 0; i < count; i++) {
            if (proposals[i].votes > best) {
                best = proposals[i].votes;
                bi = i;
            }
        }
        return bi;
    }
}
