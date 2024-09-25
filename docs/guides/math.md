# Math

Rust's primitive number types are insufficient for smart contract use cases, for three main reasons:

1. Rust only provides up to 128-bit integers, while developers often have to deal with 256- or even 512-bit integers. For example, Ethereum uses 256-bit integers to store ETH and ERC-20 balances, so if a chain has bridged assets from Ethereum, their amounts may need to be expressed in 256-bit integers. If the amounts of two such asset are to be multiplied together (which is common in AMMs), 512-bit integers may be necessary.

2. Rust does not provide [fixed-point decimal][fixed-point-arithmetic] types, which are commonly used in financial applications (we don't want to deal with precision issues with floating-point numbers such as `0.1` + `0.2` = `0.30000000000000004`). Additionally, there are concerns over [floating-point non-determinism][floating-point-determinism], hence it's often disabled in blockchains.

3. Grug uses JSON encoding for data that go in or out of contracts. However, JSON specification ([RFC 7159][rfc7159]) only guarantees support for integer numbers up to `(2**53)-1`. Any number type that may go beyond this limit needs to be serialized to JSON strings instead.

## Numbers in Grug

Grug provides a number of number types for use in smart contracts. They are built with the following two primitive types:

| type     | description         |
| -------- | ------------------- |
| `Int<U>` | integer             |
| `Dec<U>` | fixed-point decimal |

It is, however, not recommended to use these types directly. Instead, Grug exports the following type alises:

| alias     | type         | description                                                |
| --------- | ------------ | ---------------------------------------------------------- |
| `Uint64`  | `Int<u64>`   | 64-bit unsigned integer                                    |
| `Uint128` | `Int<u128>`  | 128-bit unsigned integer                                   |
| `Uint256` | `Int<U256>`  | 256-bit unsigned integer                                   |
| `Uint512` | `Int<U512>`  | 512-bit unsigned integer                                   |
| `Int64`   | `Int<i64>>`  | 64-bit signed integer                                      |
| `Int128`  | `Int<i128>>` | 128-bit signed integer                                     |
| `Int256`  | `Int<I256>>` | 256-bit signed integer                                     |
| `Int512`  | `Int<I512>>` | 512-bit signed integer                                     |
| `Udec128` | `Dec<i128>`  | 128-bit unsigned fixed-point number with 18 decimal places |
| `Udec256` | `Dec<I256>`  | 256-bit unsigned fixed-point number with 18 decimal places |
| `Dec128`  | `Dec<i128>>` | 128-bit signed fixed-point number with 18 decimal places   |
| `Dec256`  | `Dec<I256>>` | 256-bit signed fixed-point number with 18 decimal places   |

where `{U,I}{256,512}` are from the [bnum][bnum] library.

## How to use

> TODO

[bnum]: https://github.com/left-curve/bnum/tree/v0.11.0-grug
[fixed-point-arithmetic]: https://en.wikipedia.org/wiki/Fixed-point_arithmetic
[floating-point-determinism]: https://randomascii.wordpress.com/2013/07/16/floating-point-determinism/
[rfc7159]: https://datatracker.ietf.org/doc/html/rfc7159
