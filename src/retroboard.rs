use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
};

use shakmaty::{
    attacks,
    fen::Fen,
    Bitboard, Board, CastlingMode, Chess, Color,
    Color::{Black, White},
    FromSetup, Piece, PositionError, Rank, Role, Setup, Square,
};

use crate::{
    MoveKind::{EnPassant, Normal, UnPromotion, Uncapture},
    RetroPockets, UnMove, UnMoveList,
};

/// A [`shakmaty::Board`] where [`Unmove`](crate::UnMove) are played and all legal [`Unmove`](crate::UnMove) can be generated.
/// At every time the position must be legal. This does include unreachable positions, like [this position](https://lichess.org/editor/3k4/2B1B3/8/8/8/8/5N2/3K4_b_-_-_0_1).
#[derive(Clone)] // Copy?
pub struct RetroBoard {
    board: Board,
    retro_turn: Color,
    pockets: RetroPockets,
    halfmoves: u8, // Number of plies since a breaking unmove has been done.
    ep_square: Option<Square>,
}

impl RetroBoard {
    #[must_use]
    /// Returns a new [`RetroBoard`] with empty [`RetroPocket`](crate::RetroPocket) for both colors.
    pub fn new_no_pockets(fen: &str) -> Option<Self> {
        Self::new(fen, "", "")
    }

    #[must_use]
    /// Returns a new [`RetroBoard`] with defined [`RetroPocket`](crate::RetroPocket), see [`RetroPocket::from_str`](crate::RetroPocket) documentation
    /// to see which string format is expected.
    /// # Examples
    /// ```
    /// use retroboard::RetroBoard;
    /// let r = RetroBoard::new("3k4/8/8/8/8/8/8/2RKR3 w - - 0 1", "PNQ1", "7BBBB").unwrap();
    /// ```
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
        let ep_square = fen_vec
            .get(3)
            .and_then(|sq| Square::from_ascii(sq.as_bytes()).ok());
        // It doesn't make sense to initialize halfmoves from the fen, since doing unmoves.
        Some(RetroBoard {
            board,
            retro_turn,
            pockets,
            halfmoves: 0,
            ep_square,
        })
    }

    pub fn push(&mut self, m: &UnMove) {
        let moved_piece = self
            .board
            .remove_piece_at(m.from)
            .expect("Unmove: from square should contain a piece");
        self.halfmoves += 1;
        self.ep_square = None;

        if let Some(role) = m.uncapture() {
            self.halfmoves = 0;
            self.board.set_piece_at(
                m.uncapture_square().unwrap(),
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
        if m.is_en_passant() {
            self.ep_square = Some(m.from);
        }
        self.retro_turn = !self.retro_turn;
    }

    pub fn pseudo_legal_unmoves(&self, moves: &mut UnMoveList) {
        // then there is only move possible
        if let Some(sq) = self.ep_square {
            // ep square always on the third or sixth rank, so offseting is fine
            moves.push(UnMove::new(
                sq.offset(self.retro_turn.fold(8, -8)).unwrap(), // from
                sq.offset(self.retro_turn.fold(-8, 8)).unwrap(), // to
                Normal,
            ))
        } else {
            self.gen_pieces(moves);
            self.gen_unpromotion(moves);
            self.gen_pawns(moves);
            self.gen_en_passant(moves, Bitboard::FULL);
        }
    }

    /// Generate legal unmoves, which are all the pseudo legal unmoves which do not put the opponent's king in check.
    /// If the opponent's king is in check at the beginning of our turn, the only legal unmoves are those which stop it from being in check.
    #[must_use]
    pub fn legal_unmoves(&self) -> UnMoveList {
        // supposing the opponent's king is not in check at the beginning of our retro_turn
        let mut moves: UnMoveList = UnMoveList::new();
        let checkers = self.checkers(!self.retro_turn);
        let blockers = self.slider_blockers(self.us(), self.king_of(!self.retro_turn));
        let nb_checkers = checkers.count();
        match nb_checkers.cmp(&2) {
            Ordering::Greater => return moves, // no unmoves possible
            Ordering::Equal => {
                if checkers.is_subset(self.board.steppers()) {
                    return moves;
                };

                // should work if two sliders or one slider one stepper. If there is one stepper, the slider should be the furthest piece.
                let (from, furthest_checker) =
                    closest_further_square(checkers, self.king_of(!self.retro_turn));

                let from_piece = self.board.piece_at(from).unwrap();
                let target = attacks::between(self.king_of(!self.retro_turn), furthest_checker);
                // the closest piece must come into the way of the further one
                if let Some(to) =
                    (retro_attacks(from, from_piece, self.occupied()) & target).first()
                {
                    if from_piece.role != Role::Pawn {
                        moves.push(UnMove::new(from, to, Normal));
                    }
                    self.gen_en_passant(&mut moves, target);
                    self.gen_uncaptures(from, to, false, &mut moves);
                    if Bitboard::BACKRANKS.contains(from) {
                        self.gen_uncaptures(from, to, true, &mut moves);
                    };
                    // we do not check if the move itself gives check before
                    moves.retain(|m| !self.does_unmove_give_check(m));
                }
            }
            Ordering::Less => {
                // 1 or no checker.
                self.pseudo_legal_unmoves(&mut moves);
                moves.retain(|m| self.is_safe(m, blockers, checkers.first()));
            }
        }

        moves
    }

    // from shakmaty code-source
    fn slider_blockers(&self, our_pieces: Bitboard, king: Square) -> Bitboard {
        let snipers = (attacks::rook_attacks(king, Bitboard(0)) & self.board.rooks_and_queens())
            | (attacks::bishop_attacks(king, Bitboard(0)) & self.board.bishops_and_queens());

        let mut blockers = Bitboard(0);

        for sniper in snipers & our_pieces {
            let b = attacks::between(king, sniper) & self.occupied();

            if !b.more_than_one() {
                blockers.add(b);
            }
        }

        blockers
    }

    fn is_safe(&self, unmove: &UnMove, blockers: Bitboard, checker: Option<Square>) -> bool {
        let king = self.king_of(!self.retro_turn);
        // If we remove a blocker without letting a piece behing we'll put the king in check, so the unmove is invalid
        if !unmove.is_uncapture()
            && blockers.contains(unmove.from)
            && !attacks::aligned(unmove.from, unmove.to, king)
        {
            return false;
        }

        // check if the unmove attack the king
        if self.does_unmove_give_check(unmove) {
            return false;
        }

        // no checker we can end here
        if checker.is_none() {
            return true;
        }

        // if the checker does not move and is not a slider, then at the end the king will still be in check
        if self.board.steppers().contains(checker.unwrap()) && checker.unwrap() != unmove.from {
            return false;
        }
        // Now we know the checker is a slider and either it moves away to a square where it does not put the king in check (we already checked if the destination square gives check, so only left to check if it is the checker)
        // or it does not move, and then we need to check if a piece goes between it.
        checker.unwrap() == unmove.from
            || attacks::between(checker.unwrap(), king).contains(unmove.to)
    }

    fn does_unmove_give_check(&self, unmove: &UnMove) -> bool {
        (attacks::attacks(
            unmove.to,
            if unmove.is_unpromotion() {
                self.retro_turn.pawn()
            } else {
                self.board.piece_at(unmove.from).unwrap()
            },
            self.occupied()
                ^ if unmove.is_uncapture() {
                    Bitboard::EMPTY
                } else {
                    unmove.from.into()
                },
        ) & self.king_of(!self.retro_turn))
        .any()
    }

    #[inline]
    #[must_use]
    pub fn board(&self) -> &Board {
        &self.board
    }

    #[inline]
    #[must_use]
    pub fn retro_turn(&self) -> Color {
        self.retro_turn
    }

    #[inline]
    #[must_use]
    pub fn us(&self) -> Bitboard {
        self.board.by_color(self.retro_turn)
    }

    #[inline]
    #[must_use]
    pub fn our(&self, role: Role) -> Bitboard {
        self.us() & self.board.by_role(role)
    }

    #[inline]
    #[must_use]
    pub fn them(&self) -> Bitboard {
        self.board.by_color(!self.retro_turn)
    }

    #[inline]
    #[must_use]
    pub fn their(&self, role: Role) -> Bitboard {
        self.them() & self.board.by_role(role)
    }

    #[inline]
    #[must_use]
    fn occupied(&self) -> Bitboard {
        self.board.occupied()
    }

    #[inline]
    #[must_use]
    pub fn king_of(&self, color: Color) -> Square {
        self.board.king_of(color).unwrap()
    }

    #[inline]
    fn epd(&self) -> String {
        format!(
            "{} {} - {}",
            self.board.board_fen(Bitboard::EMPTY),
            match self.retro_turn {
                Black => "w",
                White => "b",
            },
            self.ep_square.map_or_else(
                || "-".to_string(),
                |sq| format! {"{:?}", sq}.to_ascii_lowercase()
            )
        )
    }

    #[inline]
    fn checkers(&self, color: Color) -> Bitboard {
        self.board
            .attacks_to(self.king_of(color), !color, self.occupied())
    }

    fn gen_unpromotion(&self, moves: &mut UnMoveList) {
        if self.pockets.color(self.retro_turn).unpromotion > 0 {
            for from in self.us() & Bitboard::relative_rank(self.retro_turn, Rank::Eighth) {
                self.gen_unpromotion_on(from, moves);
            }
        }
    }

    fn gen_unpromotion_on(&self, from: Square, moves: &mut UnMoveList) {
        let to = from
            .offset(self.retro_turn.fold(-8, 8))
            .expect("We're in the eighth rank and going back so square exists");
        if self.board.piece_at(to).is_none() {
            moves.push(UnMove::new(from, to, UnPromotion(None)));
        };
        self.gen_pawn_uncaptures(from, true, moves);
    }

    fn gen_pieces(&self, moves: &mut UnMoveList) {
        for from in self.us() & !self.our(Role::Pawn) {
            for to in attacks::attacks(from, self.board.piece_at(from).unwrap(), self.occupied())
                & !self.occupied()
            {
                moves.push(UnMove::new(from, to, Normal));
                self.gen_uncaptures(from, to, false, moves)
            }
        }
    }

    fn gen_en_passant(&self, moves: &mut UnMoveList, target: Bitboard) {
        if self.pockets.color(!self.retro_turn).pawn > 0 {
            // pawns on the relative 6th rank with free space above AND below them
            let ep_pawns = self.our(Role::Pawn)
                & Bitboard::relative_rank(self.retro_turn, Rank::Sixth)
                & (!(Bitboard::relative_rank(self.retro_turn, Rank::Fifth) & self.occupied()))
                    .relative_shift(self.retro_turn, 8)
                & (!(Bitboard::relative_rank(self.retro_turn, Rank::Seventh) & self.occupied()))
                    .relative_shift(!self.retro_turn, 8);

            for from in ep_pawns {
                for to in attacks::pawn_attacks(!self.retro_turn, from) & !self.occupied() & target
                {
                    moves.push(UnMove::new(from, to, EnPassant));
                }
            }
        }
    }

    fn gen_pawns(&self, moves: &mut UnMoveList) {
        // generate pawn uncaptures
        for from in self.our(Role::Pawn) & !Bitboard::relative_rank(self.retro_turn, Rank::Second) {
            self.gen_pawn_uncaptures(from, false, moves)
        }

        let single_moves =
            self.our(Role::Pawn).relative_shift(!self.retro_turn, 8) & !self.occupied();

        let double_moves = single_moves.relative_shift(!self.retro_turn, 8)
            & Bitboard::relative_rank(self.retro_turn, Rank::Second)
            & !self.occupied();

        for to in single_moves & !Bitboard::BACKRANKS {
            if let Some(from) = to.offset(self.retro_turn.fold(8, -8)) {
                moves.push(UnMove::new(from, to, Normal));
            }
        }

        for to in double_moves {
            if let Some(from) = to.offset(self.retro_turn.fold(16, -16)) {
                moves.push(UnMove::new(from, to, Normal));
            }
        }
    }

    fn gen_pawn_uncaptures(&self, from: Square, unpromotion: bool, moves: &mut UnMoveList) {
        for to in attacks::pawn_attacks(!self.retro_turn, from) & !self.occupied() {
            self.gen_uncaptures(from, to, unpromotion, moves)
        }
    }

    // TODO refractor uncapture to uncapture_on, dealing with attacks, unpromotion etc.
    fn gen_uncaptures(&self, from: Square, to: Square, unpromotion: bool, moves: &mut UnMoveList) {
        for unmove in self
            .pockets
            .color(!self.retro_turn)
            .clone()
            .into_iter()
            .map(|r| {
                UnMove::new(
                    from,
                    to,
                    if unpromotion {
                        UnPromotion(Some(r))
                    } else {
                        Uncapture(r)
                    },
                )
            })
        {
            if !(Bitboard::BACKRANKS.contains(unmove.from)
                && unmove.uncapture() == Some(Role::Pawn))
            {
                // pawns cannot be uncaptured on backrank
                moves.push(unmove)
            }
        }
    }
}

impl PartialEq for RetroBoard {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.retro_turn == other.retro_turn
            && self.board == other.board
            && self.pockets == other.pockets
            && self.ep_square == other.ep_square
    }
}

impl Hash for RetroBoard {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.retro_turn.hash(state);
        self.board.hash(state);
        self.pockets.hash(state);
        self.ep_square.hash(state);
    }
}

impl Eq for RetroBoard {}

impl fmt::Debug for RetroBoard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!(
            "\n{}\nretro_turn = {:?}\n{:?}\nhalfmoves: {:?}\nep square: {:?}",
            show_board(&self.board),
            self.retro_turn,
            self.pockets,
            self.halfmoves,
            self.ep_square,
        ))
    }
}

impl FromSetup for RetroBoard {
    /// [`RetroPocket`](crate::RetroPocket) will be empty for both colors
    fn from_setup(setup: &dyn Setup, _: CastlingMode) -> Result<Self, PositionError<Self>> {
        Ok(Self {
            board: setup.board().clone(),
            retro_turn: !setup.turn(),
            ep_square: setup.ep_square(),
            halfmoves: 0,
            pockets: RetroPockets::default(),
        })
    }
}

/// Consider valid positions with too many/impossible checkers (unreachable positions)
impl From<RetroBoard> for Chess {
    fn from(item: RetroBoard) -> Self {
        let setup: Fen = item
            .epd()
            .parse()
            .unwrap_or_else(|_| panic!("syntactically correct EPD: {:?}", item.epd()));

        match setup.position(CastlingMode::Standard) {
            Err(x) => x
                .ignore_impossible_check()
                .unwrap_or_else(|_| panic!("Legal Position: {}", setup)),
            Ok(pos) => pos,
        }
    }
}

#[inline]
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
        x => x,
    }
}

#[inline]
fn retro_attacks(from: Square, p: Piece, occupied: Bitboard) -> Bitboard {
    match p {
        Piece {
            color,
            role: Role::Pawn,
        } => attacks::attacks(from, (!color).pawn(), occupied),
        _ => attacks::attacks(from, p, occupied),
    }
}

#[inline]
fn show_board(board: &Board) -> String {
    let board_unicode: String = format!("{:?}", board).chars().map(unicode).collect();
    board_unicode
}

#[inline]
fn closest_further_square(bb: Bitboard, of: Square) -> (Square, Square) {
    let (sq_1, sq_2) = (bb.first().unwrap(), bb.last().unwrap());
    if sq_1.distance(of) < sq_2.distance(of) {
        (sq_1, sq_2)
    } else {
        (sq_2, sq_1)
    }
}

#[cfg(test)]
mod tests {
    // use pretty_assertions::{assert_eq, assert_ne};
    use std::collections::HashSet;

    use indoc::indoc;
    use paste::paste;
    use shakmaty::{uci::Uci, Position};

    use super::*;

    fn u(s: &str) -> UnMove {
        UnMove::from_retro_uci(s).unwrap()
    }

    #[test]
    #[allow(clippy::non_ascii_literal)]
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
                halfmoves: 0
                ep square: None"}
        )
    }

    #[test]
    fn test_new_no_pockets() {
        let r =
            RetroBoard::new_no_pockets("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .expect("Retroboard because fen is legal");
        assert_eq!(r.retro_turn, Black);
        assert_eq!(
            r.board,
            Board::from_board_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR".as_bytes())
                .unwrap()
        );
        assert_eq!(r.pockets, RetroPockets::default());
        assert_eq!(r.halfmoves, 0);
    }

    #[test]
    fn test_from_setup() {
        let r =
            RetroBoard::new_no_pockets("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .expect("Retroboard because fen is legal");
        let fen: Fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
            .parse()
            .unwrap();
        let r_setup = RetroBoard::from_setup(&fen, CastlingMode::Standard).unwrap();
        assert_eq!(r, r_setup);
    }

    #[test]
    fn test_hash() {
        let mut r =
            RetroBoard::new_no_pockets("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1")
                .unwrap();
        r.push(&u("g1f3"));
        r.push(&u("g8f6"));
        r.push(&u("f3g1"));
        r.push(&u("f6g8"));
        let mut hashset: HashSet<RetroBoard> = HashSet::new();
        hashset.insert(r.clone());
        let r2 =
            RetroBoard::new_no_pockets("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1")
                .unwrap();
        assert_ne!(r.halfmoves, r2.halfmoves);
        assert_eq!(r, r2);
        assert!(hashset.contains(&r2))
    }

    #[test]
    fn test_retro_attacks() {
        assert_eq!(
            retro_attacks(Square::E4, Black.pawn(), Bitboard::EMPTY),
            Bitboard::EMPTY | Square::D5 | Square::F5
        );
        assert_eq!(
            retro_attacks(Square::A1, Black.knight(), Bitboard::EMPTY),
            Bitboard::EMPTY | Square::B3 | Square::C2
        );
    }

    #[test]
    fn test_push_uncapture() {
        for piece in "PNBRQ".chars() {
            let mut r =
                RetroBoard::new("4k3/r7/8/8/8/8/8/4K3 w - - 0 1", &piece.to_string(), "").unwrap();
            r.push(&u(&format!("{}a7a2", piece)));
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
            r.push(&u("Ub8b7"));
            assert_eq!(
                r,
                RetroBoard::new("8/1P5k/8/8/8/8/8/1K6 w - - 0 1", &(i - 1).to_string(), "")
                    .unwrap()
            )
        }
    }

    #[test]
    fn test_push_en_passant() {
        let mut r = RetroBoard::new("k7/8/2P5/8/8/8/8/2K5 b - - 0 1", "", "P").unwrap();
        r.push(&u("Ec6d5"));
        assert_eq!(
            r,
            RetroBoard::new("k7/8/8/2pP4/8/8/8/2K5 w - c6 0 1", "", "").unwrap()
        )
    }

    #[test]
    fn test_push_unpromote_and_uncapture() {
        for piece in "NBRQ".chars() {
            let mut r =
                RetroBoard::new("r3k3/8/8/8/8/8/8/4K3 w - - 0 1", &piece.to_string(), "1").unwrap();
            r.push(&u(&format!("U{}a8b7", piece)));
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
            "w" => "b",
            _ => panic!("Turn should be either black or white"),
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

    fn move_legal(r: &RetroBoard, pos: Chess, unmove: UnMove) -> bool {
        pos.is_legal(
            &Uci::from_ascii(
                format!(
                    "{}{}{}",
                    unmove.to,
                    unmove.from,
                    if unmove.is_unpromotion() {
                        r.board
                            .piece_at(unmove.from)
                            .unwrap()
                            .role
                            .char()
                            .to_string()
                    } else {
                        "".to_string()
                    }
                )
                .as_bytes(),
            )
            .expect("Valid uci")
            .to_move(&pos)
            .expect("correct move"),
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
            let _: Chess = r.clone().into(); // check if position is legal
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
                "pseudo" => r.pseudo_legal_unmoves(&mut m2),
                "legal" => m2 = r.legal_unmoves(),
                _ => panic!("Choose proper generation method"),
            };
            for x in m2.clone() {
                assert!(!m2_hashset.contains(&x)); // check for move duplicated
                m2_hashset.insert(x.clone());
            }
            let mut gen_not_exp = m2_hashset.clone();
            let mut exp_not_gen = m1_hashset.clone();
            gen_not_exp.retain(|x| !m1_hashset.contains(x));
            exp_not_gen.retain(|x| !m2_hashset.contains(x));
            println!("{:?}", r);
            println!("Mirrored: {:?}", mirrored);
            println!("Generated but not expected: {:?}", gen_not_exp);
            println!("Expected but not generated: {:?}", exp_not_gen);
            assert_eq!(m1_hashset, m2_hashset);
            for x in m2.clone() {
                if gen_type == "legal" {
                    let mut r_after_unmove = r.clone();
                    r_after_unmove.push(&x);
                    let chess_after_unmove: Chess = r_after_unmove.into();
                    assert!(move_legal(&r, chess_after_unmove, x));
                }
            }
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
        pseudo_en_passant, "1k6/8/4P3/8/8/8/nn6/Kn6 b - - 0 1", "", "P", "pseudo", "e6e5 Pe6d5 Pe6f5 Ee6d5 Ee6f5",
        pseudo_pre_en_passant_only, "1k6/8/8/8/4P3/8/8/K7 b - e3 0 1", "", "P", "pseudo", "e4e2",
        no_en_passant_sq_blocked, "4k1b1/8/4P3/4p3/8/n7/Kn6/nn6 b - - 0 1","", "P", "pseudo", "Pe6d5 Pe6f5 a2b3 Pa2b3",
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
            let mut unmove_list_1 = UnMoveList::new();
            r.pseudo_legal_unmoves(&mut unmove_list_1);
            for m in unmove_list_1 {
                counter += 1;
                let mut r2 = r.clone();
                r2.push(&m);
                let mut unmove_list_2 = UnMoveList::new();
                r2.pseudo_legal_unmoves(&mut unmove_list_2);
                for _ in unmove_list_2 {
                    counter += 1
                }
            }
            assert_eq!(counter, 22952)
        }
    }
    // now testing legal unmoves
    gen_tests_unmoves_no_pockets! {
        giving_check_illegal, "1k5R/8/Kn6/nn5p/8/8/8/8 b - - 0 1", "legal", "h8h7 h8h6",
        blocker, "1k5R/7p/1K3N2/8/8/8/8/8 b - - 0 1", "legal", "f6e8 f6g8",
        pinned_knight, "3k1N1R/8/7p/8/8/8/8/K7 b - - 0 1", "legal", "h8g8 h8h7 a1b1 a1b2 a1a2",
        knight_checker_cant_be_blocked, "3kn3/8/3K4/8/8/8/8/q7 w - - 0 1", "legal", "e8c7 e8f6 e8f6 e8g7",
        pawn_checker_cant_be_blocked, "3k4/8/8/4p3/3K4/8/8/1q6 w - - 0 1", "legal", "e5e6 e5e7",
        checkmating_is_illegal_bc_check, "k7/1Q6/1Kb5/8/8/8/8/8 b - - 0 1", "legal", "b7c7 b7d7 b7e7 b7f7 b7g7 b7h7",
        check_illegal, "1k3R2/8/Kn6/nn3p2/8/8/8/8 b - - 0 1","legal", "f8f7 f8f6",
        double_check, "3k4/8/8/3R4/7B/8/8/4K3 b - - 0 1","legal", "d5g5",
        double_check_no_moves, "8/8/3R1k2/8/7B/8/8/4K3 b - - 0 1","legal", "",
        double_check_queen_knight, "8/4k3/2N5/8/8/4Q3/8/4K3 b - - 0 1","legal", "c6e5",
        double_check_queen_knight_impossible, "4k3/2N5/4Q3/8/8/8/8/3K4 b - - 0 1","legal", "",
        double_check_double_pawns, "4k3/3P1P2/8/8/8/8/8/3K4 b - - 0 1","legal", "",
        double_check_double_knights, "4k3/2N5/5N2/8/8/8/8/3K4 b - - 0 1","legal", "",
        double_check_knight_pawn, "4k3/2N2P2/8/8/8/8/8/3K4 b - - 0 1","legal", "",
        double_check_queens, "4kQ2/8/4Q3/8/8/8/8/3K4 b - - 0 1","legal", "",
    }

    gen_tests_unmoves! {
        unpromoting_legal_not_moving, "6nR/n1k5/Kn5p/nn6/8/8/8/8 b - - 0 1", "1", "N","legal", "Uh8h7 UNh8g7",
        uncapturing_create_a_blocker, "1k3R2/8/Kn6/nn3p2/8/8/8/8 b - - 0 1", "", "PQ","legal", "f8f7 f8f6 Qf8f7 Qf8f6 Qf8g8 Qf8h8",
        legal_pawn_uncaptures, "8/8/8/8/5k2/6P1/8/1K6 b - - 0 1", "", "PNBRQ","legal", "g3g2 Pg3f2 Pg3h2 Ng3f2 Ng3h2 Bg3f2 Bg3h2 Rg3f2 Rg3h2 Qg3f2 Qg3h2",
        unpromotion_illegal, "3kR3/8/8/8/8/8/8/3K4 b - - 0 1", "1", "","legal", "e8e7 e8e6 e8e5 e8e4 e8e3 e8e2 e8e1",
        unpromotion_uncapture, "3kR3/8/8/8/8/8/8/3K4 b - - 0 1", "1", "N","legal", "Ne8e7 Ne8e6 Ne8e5 Ne8e4 Ne8e3 Ne8e2 Ne8e1 UNe8d7 UNe8f7 Ne8f8 Ne8g8 Ne8h8 e8e1 e8e6 e8e2 e8e5 e8e7 e8e3 e8e4",
        double_check_with_uncaptures, "3k4/8/8/3R4/7B/8/8/4K3 b - - 0 1","", "PNBRQ", "legal", "d5g5 Pd5g5 Nd5g5 Bd5g5 Rd5g5 Qd5g5",
        double_check_queens_unpromotion, "4kQ2/8/4Q3/8/8/8/8/3K4 b - - 0 1","1", "PNBRQ", "legal", "UBf8e7 UNf8e7 URf8e7 UQf8e7",
        double_check_pawns, "8/8/4k3/5P2/8/8/nn2R3/Kn6 b - - 0 1","", "PNBRQ", "legal", "Pf5e4 Nf5e4 Bf5e4 Rf5e4 Qf5e4",
        triple_check, "8/1R1k2R1/8/8/8/3Q4/8/3K4 b - - 0 1","1PNQRB", "PNBRQ", "legal", "", // Works fine but illegal position according to shakmaty, so disabled the relevant flag
        en_passant_legal, "1k6/8/4P3/8/8/8/nn6/Kn6 b - - 0 1","", "P", "legal", "e6e5 Pe6d5 Pe6f5 Ee6d5 Ee6f5",
        no_en_passant_sq_blocked_legal, "4k1b1/8/4P3/4p3/8/n7/Kn6/nn6 b - - 0 1","", "P", "legal", "Pe6d5 Pe6f5 a2b3 Pa2b3",
        no_en_passant_opposite_check, "3k4/8/5P1n/6B1/5n1n/8/nn6/Kn6 b - - 0 1","", "P", "legal", "Pf6e5",
        en_passant_double_check, "8/4k3/5P2/8/8/8/nn2R3/Kn6 b - - 0 1","", "P", "legal", "Ef6e5 Pf6e5",
    }

    #[test]
    fn test_final_unmoves() {
        for mirrored in [false, true] {
            let fen = "q4N2/1p5k/8/8/6P1/4Q3/1K1PB3/7r b - - 0 1";
            let white_p = "2PNBRQ";
            let black_p = "3NBRQP";
            let mut counter: u64 = 0;
            let r = if mirrored {
                RetroBoard::new(&mirror_fen(fen), black_p, white_p)
                    .expect("Valid mirrored retroboard")
            } else {
                RetroBoard::new(fen, white_p, black_p).expect("Valid retroboard")
            };
            let _: Chess = r.clone().into(); // check if position is legal
            for m in r.legal_unmoves() {
                counter += 1;
                let mut r2 = r.clone();
                r2.push(&m);
                let chess_after_unmove: Chess = r2.clone().into();
                assert!(move_legal(&r, chess_after_unmove, m));
                for m2 in r2.legal_unmoves() {
                    counter += 1;
                    let mut r3 = r2.clone();
                    r3.push(&m2);
                    let chess_after_unmove2: Chess = r3.clone().into();
                    assert!(move_legal(&r2, chess_after_unmove2, m2));
                }
            }
            assert_eq!(counter, 3975)
        }
    }

    fn try_from(item: RetroBoard) -> Option<Chess> {
        let setup: Fen = item.epd().parse().ok()?;
        match setup.position(CastlingMode::Standard) {
            Err(x) => x.ignore_impossible_check().ok(),
            ok => ok.ok(),
        }
    }

    // does not take into account internal positions, contrary to `test_final_unmoves`
    fn perft_debug(r: RetroBoard, depth: u32) -> Option<u64> {
        if depth < 1 {
            Some(1)
        } else {
            try_from(r.clone())?; // check if position is legal
            let mut acc: u64 = 0;
            for m in r.legal_unmoves() {
                let mut r2 = r.clone();
                r2.push(&m);
                let chess_after_unmove: Chess = match try_from(r2.clone()) {
                    None => {
                        println!(
                            "depth {}, Illegal pos {:?}, move leading to it {:?}",
                            depth, r2, m
                        );
                        return None;
                    }
                    Some(pos) => pos,
                };
                assert!(move_legal(&r, chess_after_unmove, m.clone()));
                match perft_debug(r2.clone(), depth - 1) {
                    None => {
                        println!(
                            "depth {}, Illegal pos {:?}, move leading to it {:?}",
                            depth, r2, m
                        );
                        return None;
                    }
                    Some(x) => acc += x,
                };
            }
            Some(acc)
        }
    }

    #[test]
    fn test_perft_debug() {
        for mirrored in [false, true] {
            let fen = "q4N2/1p5k/8/8/6P1/4Q3/1K1PB3/7r b - - 0 1";
            let white_p = "2PNBRQ";
            let black_p = "3NBRQP";
            let r = if mirrored {
                RetroBoard::new(&mirror_fen(fen), black_p, white_p)
                    .expect("Valid mirrored retroboard")
            } else {
                RetroBoard::new(fen, white_p, black_p).expect("Valid retroboard")
            };
            assert_eq!(perft_debug(r.clone(), 0), Some(1));
            assert_eq!(perft_debug(r.clone(), 1), Some(24));
            assert_eq!(perft_debug(r.clone(), 2), Some(3951));
            // assert_eq!(perft_debug(r, 3), Some(640054))
        }
    }
}
