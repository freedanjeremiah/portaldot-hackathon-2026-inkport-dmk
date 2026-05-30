// Inheritance flattening: `Token is Base` merges Base's `owner` state var and
// `onlyOwner` modifier (and constructor) into Token.
contract Base {
    address owner;
    constructor() { owner = msg.sender; }
    modifier onlyOwner() { require(msg.sender == owner); _; }
    function getOwner() public view returns (address) { return owner; }
}

contract Token is Base {
    uint256 public total;
    function mint(uint256 n) public onlyOwner { total += n; }
}
