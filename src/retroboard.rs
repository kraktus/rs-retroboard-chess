use shakmaty::{Board, Color, Color::Black, Color::White};

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct RetroBoard {
    board: Board,
    retro_turn: Color,
    //TODO pocket_white: xxx,
    //TODO pocket_black: xxx,
}

/// A `Board` where `Unmove` are played and all legal `Unmove` can be generated, on top of every thing a `Board` can do.
/// At every time the position must be legal.
impl RetroBoard {
    pub fn new(fen: &str) -> Option<Self> {
        let fen_vec: Vec<&str> = fen.split(' ').collect();
        let retro_turn = match fen_vec.get(1).unwrap_or(&"w") {
            &"b" => Some(Black),
            &"w" => Some(White),
            _ => None,
        }?;
        let board = Board::from_board_fen(fen_vec.get(0)?.as_bytes()).ok()?;
        Some(RetroBoard { board, retro_turn })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_rboard() {
        assert!(
            RetroBoard::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").is_some()
        )
    }
}
