use shakmaty::Piece;
use shakmaty::{Board, Color, Color::Black, Color::White, File, Rank, Role, Square};
use std::fmt;

use crate::{RetroPockets, UnMove};

#[derive(Hash, Clone)] // Copy?
pub struct RetroBoard {
    board: Board,
    retro_turn: Color,
    pockets: RetroPockets,
    halfmoves: u8, // Number of plies since a breaking unmove has been done.
                   // TODO en-passant
}

/// A `Board` where `Unmove` are played and all legal `Unmove` can be generated, on top of every thing a `Board` can do.
/// At every time the position must be legal.
impl RetroBoard {
    pub fn new_no_pockets(fen: &str) -> Option<Self> {
        Self::new(fen, "", "")
    }

    pub fn new(fen: &str, pocket_white: &str, pocket_black: &str) -> Option<Self> {
        let fen_vec: Vec<&str> = fen.split(' ').collect();
        let retro_turn = match *fen_vec.get(1).unwrap_or(&"w") {
            // opposite of side to move
            "b" => Some(White),
            "w" => Some(Black),
            _ => None,
        }?;
        let board = Board::from_board_fen(fen_vec.get(0)?.as_bytes()).ok()?;
        let pockets = RetroPockets::from_str(pocket_white, pocket_black).ok()?;
        // It doesn't make sense to initialize halfmoves from the fen, since doing unmoves.
        Some(RetroBoard {
            board,
            retro_turn,
            pockets,
            halfmoves: 0,
        })
    }

    pub fn push(&mut self, m: UnMove) {
        let moved_piece = self
            .board
            .remove_piece_at(m.from)
            .expect("Unmove: from square should contain a piece");
        self.halfmoves += 1;

        if let Some(role) = m.uncapture {
            self.halfmoves = 0;
            self.board.set_piece_at(
                m.from,
                Piece {
                    role,
                    color: !self.retro_turn,
                },
            );
            self.pockets.turn(!self.retro_turn).decr(role);
        };
        if m.is_unpromotion() {
            self.halfmoves = 0;
            self.board.set_piece_at(
                m.to,
                Piece {
                    role: Role::Pawn,
                    color: self.retro_turn,
                },
            );
            self.pockets.turn(self.retro_turn).unpromotion -= 1;
        } else {
            self.board.set_piece_at(m.to, moved_piece);
        };
        self.retro_turn = !self.retro_turn;
    }
}

impl PartialEq for RetroBoard {
    fn eq(&self, other: &Self) -> bool {
        self.retro_turn == other.retro_turn
            && self.board == other.board
            && self.pockets == other.pockets
    }
}

impl Eq for RetroBoard {}

impl fmt::Debug for RetroBoard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!(
            "{}\nretro_turn = {:?}\n{:?}\nhalfmoves: {:?}",
            show_board(&self.board),
            self.retro_turn,
            self.pockets,
            self.halfmoves
        ))
    }
}

fn unicode(c: char) -> char {
    match c {
        'R' => '♖',
        'r' => '♜',
        'N' => '♘',
        'n' => '♞',
        'B' => '♗',
        'b' => '♝',
        'Q' => '♕',
        'q' => '♛',
        'K' => '♔',
        'k' => '♚',
        'P' => '♙',
        'p' => '♟',
        _ => '?',
    }
}

fn show_board(board: &Board) -> String {
    // TODO map over `Board` Debug, or implement in shakmaty
    let mut board_unicode = String::from("\n"); // start with a newline otherwise there's an off-set on the top line if writing something, eg. println!(yeah {:?}, game)
    for rank in (0..8).map(Rank::new).rev() {
        for file in (0..8).map(File::new) {
            let square = Square::from_coords(file, rank);
            board_unicode.push(
                board
                    .piece_at(square)
                    .map_or('.', |x| unicode(Piece::char(x))),
            );
            board_unicode.push(if file < File::H { ' ' } else { '\n' });
        }
    }
    board_unicode
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    fn u(s: &str) -> UnMove {
        UnMove::from_retro_uci(s).unwrap()
    }

    #[test]
    fn test_debug() {
        let r = RetroBoard::new(
            "kqrbn2k/5p2/8/8/8/8/5P2/KQRBN3 w - - 0 1",
            "2PPPNBR",
            "4PPNBBRQ",
        )
        .unwrap();
        println!("{:?}", r);
        assert_eq!(
            format!("{:?}", r),
            indoc! {"

                ♚ ♛ ♜ ♝ ♞ . . ♚
                . . . . . ♟ . .
                . . . . . . . .
                . . . . . . . .
                . . . . . . . .
                . . . . . . . .
                . . . . . ♙ . .
                ♔ ♕ ♖ ♗ ♘ . . .

                retro_turn = Black
                RetroPockets { black: \"PPNBBRQ4\", white: \"PPPNBR2\" }
                halfmoves: 0"}
        )
    }

    #[test]
    fn new_no_pockets() {
        let r =
            RetroBoard::new_no_pockets("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .expect("Retroboard bc fen is legal");
        assert_eq!(r.retro_turn, Black);
        assert_eq!(
            r.board,
            Board::from_board_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR".as_bytes())
                .unwrap()
        );
        assert_eq!(r.pockets, RetroPockets::new());
        assert_eq!(r.halfmoves, 0);
    }

    #[test]
    fn test_push_simple() {
        let mut r = RetroBoard::new_no_pockets("k7/6P1/8/8/8/8/8/3K4 b - - 0 1").unwrap();
        r.push(u("d1e2"));
        assert_eq!(
            r,
            RetroBoard::new_no_pockets("k7/6P1/8/8/8/8/4K3/8 w - - 0 1").unwrap()
        )
    }

    #[test]
    fn test_push_uncapture() {
        for piece in "PNBRQ".chars() {
            let mut r =
                RetroBoard::new("4k3/r7/8/8/8/8/8/4K3 w - - 0 1", &piece.to_string(), "").unwrap();
            r.push(u(&format!("{}a7a2", piece)));
            assert_eq!(
                r,
                RetroBoard::new_no_pockets(&format!("4k3/{}7/8/8/8/8/r7/4K3 b - - 0 1", piece))
                    .unwrap()
            )
        }
    }

    #[test]
    fn test_push_unpromote() {
        for i in 1..9 {
            let mut r =
                RetroBoard::new("1R6/7k/8/8/8/8/8/1K6 b - - 0 1", &i.to_string(), "").unwrap();
            r.push(u("Ub8b7"));
            assert_eq!(
                r,
                RetroBoard::new("8/1P5k/8/8/8/8/8/1K6 w - - 0 1", &(i - 1).to_string(), "")
                    .unwrap()
            )
        }
    }

    #[test]
    fn test_push_unpromote_and_uncapture() {
        for piece in "NBRQ".chars() {
            let mut r =
                RetroBoard::new("r3k3/8/8/8/8/8/8/4K3 w - - 0 1", &piece.to_string(), "1").unwrap();
            r.push(u(&format!("U{}a8b7", piece)));
            assert_eq!(
                r,
                RetroBoard::new_no_pockets(&format!("{}3k3/1p6/8/8/8/8/8/4K3 b - - 0 2", piece))
                    .unwrap()
            )
        }
    }
}
