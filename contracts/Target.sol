// Cross-contract call target: a simple value store.
contract Target {
    uint256 value;
    constructor(uint256 v) { value = v; }
    function getValue() public view returns (uint256) { return value; }
    function setValue(uint256 v) public { value = v; }
    function addValue(uint256 d) public returns (uint256) { value = value + d; return value; }
}
