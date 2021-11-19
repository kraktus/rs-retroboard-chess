use shakmaty::Piece;
use shakmaty::{Board, Color, Color::Black, Color::White, Role};

use crate::{RetroPockets, UnMove};

#[derive(Hash, Eq, PartialEq, Clone, Debug)] // Copy?
pub struct RetroBoard {
    board: Board,
    retro_turn: Color,
    pockets: RetroPockets,
    // TODO en-passant, and halfmove clock.
}

/// A `Board` where `Unmove` are played and all legal `Unmove` can be generated, on top of every thing a `Board` can do.
/// At every time the position must be legal.
impl RetroBoard {
    pub fn new(fen: &str, pocket_white: &str, pocket_black: &str) -> Option<Self> {
        let fen_vec: Vec<&str> = fen.split(' ').collect();
        let retro_turn = match fen_vec.get(1).unwrap_or(&"w") {
            &"b" => Some(Black),
            &"w" => Some(White),
            _ => None,
        }?;
        let board = Board::from_board_fen(fen_vec.get(0)?.as_bytes()).ok()?;
        let pockets = RetroPockets::from_str(pocket_white, pocket_black).ok()?;
        Some(RetroBoard {
            board,
            retro_turn,
            pockets,
        })
    }

    pub fn push(&mut self, m: UnMove) {
        let moved_piece = self
            .board
            .remove_piece_at(m.from)
            .expect("Unmove: from square should contain a piece");
        self.board.set_piece_at(m.to, moved_piece);
        if let Some(role) = m.uncapture {
            self.board.set_piece_at(
                m.from,
                Piece {
                    role,
                    color: !self.retro_turn,
                },
            )
        };
        if m.is_unpromotion() {
            self.board.set_piece_at(
                m.from,
                Piece {
                    role: Role::Pawn,
                    color: !self.retro_turn,
                },
            )
        }
        // TODO en-passant, and halfmove clock.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_rboard() {
        assert!(RetroBoard::new(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "",
            ""
        )
        .is_some())
    }
}
