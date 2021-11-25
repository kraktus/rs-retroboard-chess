use std::time::Instant;

use retroboard::RetroBoard;
/// From shakmaty code source
/// Counts legal move paths of a given length.
///
/// Shorter paths (due to mate or stalemate) are not counted.
/// Computing perft numbers is useful for comparing, testing and
/// debugging move generation correctness and performance.
///
/// The method used here is simply recursively enumerating the entire tree of
/// legal moves. While this is fine for testing there is much
/// faster specialized software.
///
/// Warning: Computing perft numbers can take a long time, even at moderate
/// depths. The simple recursive algorithm can also overflow the stack at
/// high depths, but this will only come into consideration in the rare case
/// that high depths are feasible at all.
fn perft(r: &RetroBoard, depth: u32) -> u64 {
    if depth < 1 {
        1
    } else {
        let moves = r.legal_unmoves();

        if depth == 1 {
            moves.len() as u64
        } else {
            moves
                .iter()
                .map(|m| {
                    let mut child = r.clone();
                    child.push(m);
                    perft(&child, depth - 1)
                })
                .sum()
        }
    }
}

fn main() {
    let r = RetroBoard::new(
        "q4N2/1p5k/8/8/6P1/4Q3/1K1PB3/7r b - - 0 1",
        "2PNBRQ",
        "3NBRQP",
    )
    .unwrap();
    let start = Instant::now();
    let depth = 4;
    let leaves = perft(&r, depth);
    println!(
        "Perft at  depth {}, {} leaves, {:?}s",
        depth,
        leaves,
        start.elapsed()
    );
}
