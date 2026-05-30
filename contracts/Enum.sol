// Enum lowered to uint8 numeric storage.
contract Enum {
    enum Status { Pending, Active, Closed }
    Status status;

    function activate() public { status = Status.Active; }
    function close() public { status = Status.Closed; }
    function statusCode() public view returns (uint256) { return uint256(status); }
    function isActive() public view returns (bool) { return status == Status.Active; }
}
