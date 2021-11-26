# rs-Retroboard

[![crates.io](https://img.shields.io/crates/v/retroboard.svg)](https://crates.io/crates/retroboard)
[![docs.rs](https://docs.rs/retroboard/badge.svg)](https://docs.rs/retroboard)

A chess retrograde move generator. Rust port of [Retroboard](https://github.com/kraktus/retroboard-chess). Suitable for endgame tablebase generation.

## Status

Strong test suite but lack of comparaison of perft result against a trusted source.

## Specification

En-passant is supported, but not castling. Legal but unreachable positions are supported (mainly positions with too many checkers).

Examples of accepted unreachable positions:
* `8/4k3/3B1B2/8/8/8/8/4K3 b - - 0 1` Impossible check
* `8/8/R4k2/4p3/8/8/8/4K3 b - e6 0 1` Impossible en passant square. e7e5 would have been illegal because black already in check.

It aims to follow the same generation rules as used by the generation software of syzygy and Gaviota tablebase. 

## Performance

A very rough perft test at depth 4 on this position gives 88148797 moves in ~2s (tested on Apple M1). That is roughly 7x times slower than `shakmaty` crate, but is ought to be improved.

![](https://github.com/kraktus/rs-retroboard-chess/blob/master/assets/perft.svg)
<!-- <img src="https://github.com/kraktus/rs-retroboard-chess/blob/master/assets/perft.svg" alt="Perft position" width="250"/> -->

fen : `q4N2/1p5k/3P1b2/8/6P1/4Q3/3PB1r1/2KR4 b - - 0 1`, with `2PNBRQ` in white pocket, `3NBRQP` in black one, `Q` uncastling and allowing en-passant moves.


## Example

```rust
use retroboard::RetroBoard;

let r = RetroBoard::new_no_pockets("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
assert_eq!(r.legal_unmoves().len(), 4);
```