# rs-Retroboard

A chess retrograde move generator. Rust port of [Retroboard](https://github.com/kraktus/retroboard-chess). Suitable for endgame tablebase generation.

## Status

Strong test suite but lack of comparaison of perft result against a trusted source.

## Specification

En-passant is supported, but not castling. Legal but unreachable positions are supported (mainly positions with too many checkers).

## Performance

A very rough perft test at depth 4 on this position gives 88148797 moves on depth 3 in ~2s (tested on Apple M1). That is roughly 7x times slower than `shakmaty` crate, but is ought to be improved.

![](https://github.com/kraktus/rs-retroboard-chess/blob/master/assets/perft.svg)
<!-- <img src="https://github.com/kraktus/rs-retroboard-chess/blob/master/assets/perft.svg" alt="Perft position" width="250"/> -->

fen : `q4N2/1p5k/3P1b2/8/6P1/4Q3/3PB1r1/2KR4 b - - 0 1`, with `2PNBRQ` in white pocket, `3NBRQP` in black one, `Q` uncastling and allowing en-passant moves.


## Example

```rust
use retroboard::RetroBoard;

let r = RetroBoard::new_no_pockets("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
assert_eq!(r.legal_unmoves().len(), 4);
```