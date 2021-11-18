use lazy_static::lazy_static;
use regex::{Match, Regex};
use shakmaty::{Role, Square};
use std::str::FromStr;

/// Error when parsing an invalid retro UCI.
#[derive(Clone, Debug)]
pub struct ParseRetroUciError;

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum SpecialMove {
    EnPassant,
    Unpromotion,
}

impl FromStr for SpecialMove {
    type Err = ParseRetroUciError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "U" => Ok(SpecialMove::Unpromotion),
            "E" => Ok(SpecialMove::EnPassant),
            _ => Err(ParseRetroUciError),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct UnMove {
    from: Square,
    to: Square,
    uncapture: Option<Role>,
    special_move: Option<SpecialMove>,
}

impl UnMove {
    /// movements are represented with uci, but for uncapture and unpromote
    /// a special syntax is used:
    /// -Uncapture: the piece left at the source square is indicated at the beginning, follow by normal uci move.
    /// e.g: "Re2e4" the piece on e2 goes on e4 and leaves a Rook from the opposite color on e2.
    /// -Unpromotion: "U" and after the square from which the piece will underpromote and the
    /// source square must be on the 8th or 1st rank, and dest square must be on first or second rank.
    /// e.g: "Ue8e7".
    /// An unpromotion can also be an uncapture, in this case it's noted "<PieceType>U<from_square><to_square>"
    /// e.g "UNe8e7"
    /// -En passant: "E" then the source square of the pawn and the destination of it.
    /// When a move is en-passsant, it cannot Uncapture anything (since the pawn uncapture is already implied)
    /// e.g "Ed6e5". Note than it's different than "Pd6e5". In the first example, the uncaptured pawn is in `d5`,
    /// while in the second one it's in `d6`.

    /// regex: r"[UE]?[NBRQ]?([abcdefgh][1-8]){2}"

    /// Note: A unmove being accepted does not means it is for sure legal, just syntaxically correct
    fn from_retro_uci(retro_uci: &str) -> Option<UnMove> {
        lazy_static! {
        static ref UNMOVE_REGEX: Regex = Regex::new(r"^(?P<special_move>[UE]?)(?P<uncapture>[PNBRQ]?)(?P<from>([abcdefgh][1-8])(?P<to>([abcdefgh][1-8]))$").unwrap();
        }
        UNMOVE_REGEX.captures(retro_uci).and_then(|cap| {
            Some(UnMove {
                from: cap
                    .name("from")
                    .map(|x| Square::from_ascii(x.as_str().as_bytes()).ok())
                    .flatten()?,
                to: cap
                    .name("to")
                    .map(|x| Square::from_ascii(x.as_str().as_bytes()).ok())
                    .flatten()?,
                uncapture: cap
                    .name("uncapture")
                    .map(|x| Role::from_char(x.as_str().chars().next().unwrap()))?,
                special_move: cap
                    .name("special_move")
                    .and_then(|x| SpecialMove::from_str(x.as_str()).ok()),
            })
        })
    }
}
