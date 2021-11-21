use shakmaty::Piece;
use shakmaty::{
    attacks, Bitboard, Board, Color, Color::Black, Color::White, File, Rank, Role, Square,
};
use std::fmt;

use crate::{RetroPockets, SpecialMove, UnMove, UnMoveList};

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
            self.pockets.color_mut(!self.retro_turn).decr(role);
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
            self.pockets.color_mut(self.retro_turn).unpromotion -= 1;
        } else {
            self.board.set_piece_at(m.to, moved_piece);
        };
        self.retro_turn = !self.retro_turn;
    }

    pub fn generate_pseudo_legal_unmoves(&self) -> UnMoveList {
        let mut moves = UnMoveList::new(); // TODO
        moves
    }

    fn us(&self) -> Bitboard {
        self.board.by_color(self.retro_turn)
    }

    fn our(&self, role: Role) -> Bitboard {
        self.us() & self.board.by_role(role)
    }

    fn them(&self) -> Bitboard {
        self.board.by_color(!self.retro_turn)
    }

    fn their(&self, role: Role) -> Bitboard {
        self.them() & self.board.by_role(role)
    }

    fn gen_pawns(&self, target: Bitboard, moves: &mut UnMoveList) {
        // generate pawn uncaptures
        for from in self.our(Role::Pawn) & !Bitboard::relative_rank(self.retro_turn, Rank::Second) {
            for to in
                attacks::pawn_attacks(!self.retro_turn, from) & !self.board.occupied() & target
            {
                self.gen_uncaptures(from, to, false, moves)
            }
        }

        let single_moves =
            self.our(Role::Pawn).relative_shift(!self.retro_turn, 8) & !self.board.occupied();

        let double_moves = single_moves.relative_shift(!self.retro_turn, 8)
            & Bitboard::relative_rank(self.retro_turn, Rank::Second)
            & !self.board.occupied();

        for to in single_moves & target & !Bitboard::BACKRANKS {
            if let Some(from) = to.offset(self.retro_turn.fold(8, -8)) {
                moves.push(UnMove {
                    from,
                    to,
                    uncapture: None,
                    special_move: None,
                });
            }
        }

        for to in double_moves & target {
            if let Some(from) = to.offset(self.retro_turn.fold(16, -16)) {
                moves.push(UnMove {
                    from,
                    to,
                    uncapture: None,
                    special_move: None,
                });
            }
        }
    }

    fn gen_uncaptures(&self, from: Square, to: Square, unpromotion: bool, moves: &mut UnMoveList) {
        for unmove in self
            .pockets
            .color(!self.retro_turn)
            .clone()
            .into_iter()
            .map(|r| UnMove {
                from,
                to,
                uncapture: Some(r),
                special_move: if unpromotion {
                    Some(SpecialMove::UnPromotion)
                } else {
                    None
                },
            })
        {
            moves.push(unmove)
        }
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
    // TODO map over `Board` Debug, or implement it in shakmaty (pretty-print?)
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

    fn check_moves(fen: &str, gen_type: &str, moves: &str) {
        let r = RetroBoard::new_no_pockets(fen).unwrap();
        let mut m1 = UnMoveList::new();
        let mut m2 = UnMoveList::new();
        for x in moves.split(' ') {
            m1.push(u(x))
        }
        match gen_type {
            "pawns" => r.gen_pawns(Bitboard::FULL, &mut m2),
            _ => r.gen_pawns(Bitboard::FULL, &mut m2),
        };
        assert_eq!(m1, m2)
    }

    // macro for generating tests
    macro_rules! gen_tests_unmoves {
    ($($fn_name:ident, $fen:tt, $gen_type:tt, $moves:tt,)+) => {
        $(
            #[test]
            fn $fn_name() {
                check_moves($fen, $gen_type, $moves);
            }
        )+
    }
}

    gen_tests_unmoves! {
        test_simple_pawn, "2k5/8/8/5P2/8/8/8/K7 b - - 0 1", "pawn", "f5f4",
        test_double_pawn, "2k5/8/8/8/5P2/8/nn6/Kn6 b - - 0 1", "pawn", "f4f3 f4f2",
    }

    #[test]
    fn test_generate_simple_pawn_unmoves() {
        let r = RetroBoard::new_no_pockets("2k5/8/8/5P2/8/8/8/K7 b - - 0 1").unwrap();
        let mut m1 = UnMoveList::new();
        let mut m2 = UnMoveList::new();
        let expected_unmoves = ["f5f4"];
        for x in expected_unmoves {
            m1.push(u(x))
        }
        r.gen_pawns(Bitboard::FULL, &mut m2);
        assert_eq!(m1, m2)
    }

    #[test]
    fn test_generate_simple_and_double_pawn_unmoves() {
        let r = RetroBoard::new_no_pockets("2k5/8/8/8/5P2/8/nn6/Kn6 b - - 0 1").unwrap();
        let mut m1 = UnMoveList::new();
        let mut m2 = UnMoveList::new();
        let expected_unmoves = ["f4f3", "f4f2"];
        for x in expected_unmoves {
            m1.push(u(x))
        }
        r.gen_pawns(Bitboard::FULL, &mut m2);
        assert_eq!(m1, m2)
    }
}
