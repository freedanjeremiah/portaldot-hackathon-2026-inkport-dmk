// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Cast {
    uint8 s;

    // Narrowing cast in a returned value: uint8(x) truncates modulo 256.
    function downRet(uint256 x) public pure returns (uint8) {
        return uint8(x);
    }

    // Narrowing cast into a narrow storage slot.
    function store(uint256 x) public {
        s = uint8(x);
    }

    function get() public view returns (uint8) {
        return s;
    }

    // Widening cast is unchanged (identity).
    function widen(uint8 a) public pure returns (uint256) {
        return uint256(a);
    }

    // Signed narrowing cast: int8(x) reinterprets the low 8 bits.
    function downSigned(int256 x) public pure returns (int8) {
        return int8(x);
    }
}
