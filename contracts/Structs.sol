// Struct locals + struct field access + struct returns-via-fields.
contract Structs {
    struct Point { uint256 x; uint256 y; }

    function sum(uint256 a, uint256 b) public pure returns (uint256) {
        Point memory p = Point(a, b);
        return p.x + p.y;
    }

    function swapFirst(uint256 a, uint256 b) public pure returns (uint256) {
        Point memory p = Point(a, b);
        p.x = b;
        p.y = a;
        return p.x;
    }
}
