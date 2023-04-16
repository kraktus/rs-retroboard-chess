use std::time::Instant;

use retroboard::shakmaty::{fen::Fen, perft as shakmaty_perft, CastlingMode, Chess};
use retroboard::{perft, RetroBoard};

fn _shakmaty(fen: &str) {
    let pos: Chess = fen
        .parse::<Fen>()
        .unwrap()
        .into_position(CastlingMode::Standard)
        .unwrap();
    let shakmaty_start = Instant::now();
    let shakmaty_depth = 6;
    let shakmaty_leaves = shakmaty_perft(&pos, shakmaty_depth);
    let shakmaty_stop = shakmaty_start.elapsed();
    println!(
        "fen {}\nShakmaty perft at  depth {}, {} leaves, {:?}, ratio {} pos/s",
        fen,
        shakmaty_depth,
        shakmaty_leaves,
        shakmaty_start.elapsed(),
        shakmaty_leaves as u128 / shakmaty_stop.as_millis() * 1000
    );
}

fn retroboard(fen: &str) {
    let r = RetroBoard::new(fen, "2PNBRQ", "3NBRQP").unwrap();
    let start = Instant::now();
    let depth = 4;
    let leaves = perft(&r, depth);
    let stop = start.elapsed();
    println!(
        "fen {}\nPerft at  depth {}, {} leaves, {:?}, ratio {} pos/s",
        fen,
        depth,
        leaves,
        start.elapsed(),
        leaves as u128 / stop.as_millis() * 1000
    );
}

fn main() {
    for fen in [
        "q4N2/1p5k/8/8/6P1/4Q3/1K1PB3/7r b - - 0 1",
        "8/PPPPPPPP/3k4/8/8/3K4/pppppppp/8 b - - 0 1",
        "q7/4kr2/8/2b4n/4K3/6N1/1R1QB3/8 w - - 0 1",
    ] {
        retroboard(fen);
        //_shakmaty(fen);
    }
}
