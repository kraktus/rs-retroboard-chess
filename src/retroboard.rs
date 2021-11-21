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
        let mut moves = UnMoveList::new();
        self.gen_pieces(&mut moves);
        self.gen_unpromotion(&mut moves);
        self.gen_pawns(&mut moves);
        moves
    }

    /// Generate legal unmoves, which are all the pseudo legal unmoves which do not put the opponent's king in check.
    /// If the opponent's king is in check at the beginning of our turn, the only legal unmoves are those which stop it from being in check.
    pub fn generate_legal_unmoves(&self) -> UnMoveList {
        // supposing the opponent's king is not in check at the beginning of our retro_turn
        let mut moves: UnMoveList = self.generate_pseudo_legal_unmoves();
        moves.retain(|m| self.is_safe(m));
        moves
    }

    fn is_safe(&self, unmove: &UnMove) -> bool {
        (attacks::attacks(
            unmove.to,
            self.board.piece_at(unmove.from).unwrap(),
            self.board.occupied() ^ unmove.from,
        ) & self.board.king_of(!self.retro_turn).unwrap())
        .is_empty()
    }

    #[inline]
    fn us(&self) -> Bitboard {
        self.board.by_color(self.retro_turn)
    }

    #[inline]
    fn our(&self, role: Role) -> Bitboard {
        self.us() & self.board.by_role(role)
    }

    #[inline]
    fn them(&self) -> Bitboard {
        self.board.by_color(!self.retro_turn)
    }

    #[inline]
    fn their(&self, role: Role) -> Bitboard {
        self.them() & self.board.by_role(role)
    }

    fn gen_unpromotion(&self, moves: &mut UnMoveList) {
        if self.pockets.color(self.retro_turn).unpromotion > 0 {
            for from in self.us() & Bitboard::relative_rank(self.retro_turn, Rank::Eighth) {
                let to = from
                    .offset(self.retro_turn.fold(-8, 8))
                    .expect("We're in the eighth rank and going back so square exists");
                if !self.board.piece_at(to).is_some() {
                    moves.push(UnMove {
                        from,
                        to,
                        uncapture: None,
                        special_move: Some(SpecialMove::UnPromotion),
                    });
                };
                self.gen_pawn_uncaptures(from, true, moves);
            }
        }
    }

    fn gen_pieces(&self, moves: &mut UnMoveList) {
        for from in self.us() & !self.our(Role::Pawn) {
            for to in attacks::attacks(
                from,
                self.board.piece_at(from).unwrap(),
                self.board.occupied(),
            ) & !self.board.occupied()
            {
                moves.push(UnMove {
                    from,
                    to,
                    uncapture: None,
                    special_move: None,
                });
                self.gen_uncaptures(from, to, false, moves)
            }
        }
    }

    fn gen_pawns(&self, moves: &mut UnMoveList) {
        // generate pawn uncaptures
        for from in self.our(Role::Pawn) & !Bitboard::relative_rank(self.retro_turn, Rank::Second) {
            self.gen_pawn_uncaptures(from, false, moves)
        }

        let single_moves =
            self.our(Role::Pawn).relative_shift(!self.retro_turn, 8) & !self.board.occupied();

        let double_moves = single_moves.relative_shift(!self.retro_turn, 8)
            & Bitboard::relative_rank(self.retro_turn, Rank::Second)
            & !self.board.occupied();

        for to in single_moves & !Bitboard::BACKRANKS {
            if let Some(from) = to.offset(self.retro_turn.fold(8, -8)) {
                moves.push(UnMove {
                    from,
                    to,
                    uncapture: None,
                    special_move: None,
                });
            }
        }

        for to in double_moves {
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

    fn gen_pawn_uncaptures(&self, from: Square, unpromotion: bool, moves: &mut UnMoveList) {
        for to in attacks::pawn_attacks(!self.retro_turn, from) & !self.board.occupied() {
            self.gen_uncaptures(from, to, unpromotion, moves)
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
            if !(Bitboard::BACKRANKS.contains(unmove.from) && unmove.uncapture == Some(Role::Pawn))
            {
                // pawns cannot be uncaptured on backrank
                moves.push(unmove)
            }
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
    use paste::paste;
    use pretty_assertions::{assert_eq, assert_ne};
    use std::collections::HashSet;

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

    fn ascii_swap_case(s: &str) -> String {
        let mut v: Vec<u8> = vec![];
        for b in s.as_bytes() {
            if let b'a'..=b'z' | b'A'..=b'Z' = b {
                v.push(b ^ 0b0010_0000);
            } else {
                v.push(*b)
            }
        }
        String::from_utf8(v).unwrap()
    }

    fn mirror_square(sq: &str) -> String {
        let v = sq.as_bytes().to_vec();
        // first byte is for the column, left unchanged
        let mut mirrored_v = vec![v[0]];
        // second byte is the rank, which needs to be mirrored.
        // square goes from 1 to 8, which is from 49 to 56 in ascii code
        mirrored_v.push(105 - v[1]);
        String::from_utf8(mirrored_v).unwrap()
    }

    /// try to "mirror" a fen, it's the caller responsability to ensure the fen is properly formated.
    /// It shoulf be faster to manipulate the fen than to reverse the board, but would need to be confirmed at some point
    fn mirror_fen(fen: &str) -> String {
        // "r1bq1r2/pp2n3/4N2k/3pPppP/1b1n2Q1/2N5/PP3PP1/R1B1K2R w KQ g6 0 15"
        // board turn castle en_passant half_moves full_moves
        let fen_vec: Vec<&str> = fen.split(' ').collect();
        let color = match fen_vec[1] {
            "b" => "w",
            "w" | _ => "b",
        };
        // swap the ranks and color of pieces
        let mirrored_board =
            ascii_swap_case(&fen_vec[0].split('/').rev().collect::<Vec<_>>().join("/"));
        let mirrored_castle = ascii_swap_case(fen_vec[2]);
        let mirrored_en_passant = match fen_vec[3] {
            "-" => "-".to_string(),
            sq_str => mirror_square(sq_str),
        };
        format!(
            "{} {} {} {} {} {}",
            mirrored_board,
            color,
            mirrored_castle,
            mirrored_en_passant,
            fen_vec.get(4).unwrap_or(&"0"),
            fen_vec.get(5).unwrap_or(&"1")
        )
    }

    fn check_moves(fen: &str, white_p: &str, black_p: &str, gen_type: &str, moves: &str) {
        for mirrored in [false, true] {
            let r = if mirrored {
                RetroBoard::new(&mirror_fen(fen), black_p, white_p)
                    .expect("Valid mirrored retroboard")
            } else {
                RetroBoard::new(fen, white_p, black_p).expect("Valid retroboard")
            };
            let mut m1_hashset: HashSet<UnMove> = HashSet::new();
            let mut m2_hashset: HashSet<UnMove> = HashSet::new();
            let mut m2 = UnMoveList::new();
            for x in moves.split(' ') {
                println!("{:?}", x);
                if !x.is_empty() {
                    m1_hashset.insert(if mirrored { u(x).mirror() } else { u(x) });
                }
            }
            match gen_type {
                "pawn" => r.gen_pawns(&mut m2),
                "piece" => r.gen_pieces(&mut m2),
                "unpromotion" => r.gen_unpromotion(&mut m2),
                "pseudo" => m2 = r.generate_pseudo_legal_unmoves(),
                "legal" | _ => m2 = r.generate_legal_unmoves(),
            };
            for x in m2 {
                m2_hashset.insert(x);
            }
            let mut gen_not_exp = m2_hashset.clone();
            let mut exp_not_gen = m1_hashset.clone();
            gen_not_exp.retain(|x| !m1_hashset.contains(x));
            exp_not_gen.retain(|x| !m2_hashset.contains(x));
            println!("Mirrored: {:?}", mirrored);
            println!("Generated but not expected: {:?}", gen_not_exp);
            println!("Expected but not generated: {:?}", exp_not_gen);
            assert_eq!(m1_hashset, m2_hashset)
        }
    }

    // macro for generating tests
    macro_rules! gen_tests_unmoves {
    ($($fn_name:ident, $fen:tt, $white_p:tt, $black_p:tt, $gen_type:tt, $moves:tt,)+) => {
        $(
            paste! {
            #[test]
            fn [<test_ $fn_name>]() {
                check_moves($fen, $white_p, $black_p, $gen_type, $moves);
            }
        }
        )+
    }
}

    // macro for generating tests
    macro_rules! gen_tests_unmoves_no_pockets {
    ($($fn_name:ident, $fen:tt, $gen_type:tt, $moves:tt,)+) => {
        $(
            gen_tests_unmoves! {$fn_name, $fen, "", "", $gen_type, $moves,}
        )+
    }
}

    gen_tests_unmoves_no_pockets! {
        simple_pawn, "2k5/8/8/5P2/8/8/8/K7 b - - 0 1", "pawn", "f5f4",
        double_pawn, "2k5/8/8/8/5P2/8/nn6/Kn6 b - - 0 1", "pawn", "f4f3 f4f2",
        no_pawn, "1k6/8/8/8/8/8/3P2nn/6nK b - - 0 1", "pawn", "",
        king, "1k6/8/8/8/8/8/nn6/Kn6 b - - 0 1", "piece", "",
        knight, "1k6/8/8/8/8/5N2/nn6/Kn6 b - - 0 1", "piece", "f3e1 f3g1 f3h2 f3h4 f3g5 f3e5 f3d4 f3d2",
        bishop, "1k6/8/8/8/3r4/8/nn3B2/Kn6 b - - 0 1", "piece", "f2e1 f2g1 f2g3 f2h4 f2e3",
        rook, "1k6/8/8/8/8/5nnn/nn3n2/Kn3n1R b - - 0 1", "piece", "h1h2 h1g1",
        queen, "1k6/8/8/8/8/5nnn/nn3n2/Kn3n1Q b - - 0 1", "piece", "h1h2 h1g1 h1g2",
    }

    gen_tests_unmoves! {
        pawn_uncapture, "3k4/8/8/8/4K3/7P/8/8 b - - 0 1", "", "PNBRQ", "pawn", "h3h2 Ph3g2 Nh3g2 Bh3g2 Rh3g2 Qh3g2",
        no_pawn_uncapture, "2k5/8/8/8/5P2/4q1q1/nn6/Kn6 b - - 0 1", "", "PNBRQ", "pawn", "f4f3 f4f2",
        rook_uncapture, "1k6/8/8/8/8/5nnn/nn3n2/Kn3n1R b - - 0 1", "", "PBNRQ", "piece", "h1h2 h1g1 Bh1h2 Bh1g1 Nh1h2 Nh1g1 Rh1h2 Rh1g1 Qh1h2 Qh1g1",
        queen_uncapture, "1k6/8/8/8/8/5nnn/nn3n2/Kn3n1Q b - - 0 1", "", "PN", "piece", "h1h2 h1g1 h1g2 Nh1h2 Nh1g1 Nh1g2",
        bishop_uncapture, "1k6/8/8/8/8/5nnn/nn3n2/Kn3n1B b - - 0 1", "", "PN", "piece", "h1g2 Nh1g2",
        knight_uncapture, "1k6/8/8/8/8/8/nn6/Kn5N b - - 0 1", "", "PQ", "piece", "h1g3 h1f2 Qh1g3 Qh1f2",
        knight_uncapture_with_pawns, "k7/8/8/8/8/8/nn5N/Kn6 b - - 0 1", "", "PQ", "piece", "h2g4 h2f3 h2f1 Qh2g4 Qh2f3 Qh2f1 Ph2g4 Ph2f3 Ph2f1",
        unpromotion_and_unpromotion_uncapture, "6N1/k3n3/5n1n/8/8/8/nn6/Kn6 b - - 0 1", "1", "PR", "unpromotion", "Ug8g7 URg8f7 URg8h7",
        unpromotion_but_uncapture_not_possible, "6N1/k3n3/5n1n/8/8/8/nn6/Kn6 b - - 0 1", "1", "", "unpromotion", "Ug8g7",
        no_unpromotion, "6N1/k3n3/5n1n/8/8/8/nn6/Kn6 b - - 0 1", "", "PQ", "unpromotion", "",
        pseudo_legal, "5BN1/k3n3/5n1n/8/5P2/8/nn6/K7 b - - 0 1", "1", "PQ", "pseudo", "a1b1 Qa1b1 Ug8g7 UQg8f7 UQg8h7 Uf8f7 UQf8g7 Qf8g7 f8g7 f4f2 f4f3 Pf4g3 Pf4e3 Qf4g3 Qf4e3",
    }

    #[test]
    fn test_final_pseudo_unmoves() {
        for mirrored in [false, true] {
            let fen = "1N6/1r5k/8/8/2P5/8/1Q2P3/n5Kb w - - 0 1";
            let black_p = "3NBRQP";
            let white_p = "2PNBRQ";
            let mut counter: u32 = 0;
            let r = if mirrored {
                RetroBoard::new(&mirror_fen(fen), black_p, white_p)
                    .expect("Valid mirrored retroboard")
            } else {
                RetroBoard::new(fen, white_p, black_p).expect("Valid retroboard")
            };
            for m in r.generate_pseudo_legal_unmoves() {
                counter += 1;
                let mut r2 = r.clone();
                r2.push(m);
                for _ in r2.generate_pseudo_legal_unmoves() {
                    counter += 1
                }
            }
            assert_eq!(counter, 22952)
        }
    }
}
