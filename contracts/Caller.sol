// Cross-contract caller. The `ITarget` interface declares the callee's ABI; the
// translator computes keccak-4 selectors for it and emits `seal_call`.
interface ITarget {
    function getValue() external view returns (uint256);
    function setValue(uint256 v) external;
    function addValue(uint256 d) external returns (uint256);
}

contract Caller {
    function readValue(address t) public view returns (uint256) {
        return ITarget(t).getValue();
    }

    function pushValue(address t, uint256 v) public {
        ITarget(t).setValue(v);
    }
}
