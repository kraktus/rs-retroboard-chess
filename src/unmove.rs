use arrayvec::ArrayVec;
use lazy_static::lazy_static;
use regex::Regex;
use shakmaty::{Role, Square};
use std::fmt;
use std::str::FromStr;

pub type UnMoveList = ArrayVec<UnMove, 512>; // TODO check if reducing that number is possible (256 used for std in shakmaty)

/// Error when parsing an invalid retro UCI.
#[derive(Clone, Debug)]
pub struct ParseRetroUciError;

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum SpecialMove {
    EnPassant,
    UnPromotion,
}

impl FromStr for SpecialMove {
    type Err = ParseRetroUciError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "U" => Ok(SpecialMove::UnPromotion),
            "E" => Ok(SpecialMove::EnPassant),
            _ => Err(ParseRetroUciError),
        }
    }
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct UnMove {
    pub from: Square,
    pub to: Square,
    pub uncapture: Option<Role>,
    pub special_move: Option<SpecialMove>,
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
    ///
    /// regex: r"[UE]?[NBRQ]?([abcdefgh][1-8]){2}"
    ///
    /// Note: A unmove being accepted does not means it is for sure legal, just syntaxically correct
    pub fn from_retro_uci(retro_uci: &str) -> Result<UnMove, ParseRetroUciError> {
        lazy_static! {
        static ref UNMOVE_REGEX: Regex = Regex::new(r"^(?P<special_move>[UE]?)(?P<uncapture>[PNBRQ]?)(?P<from>([abcdefgh][1-8]))(?P<to>([abcdefgh][1-8]))$").unwrap();
        }
        UNMOVE_REGEX
            .captures(retro_uci)
            .and_then(|cap| {
                Some(UnMove {
                    from: cap
                        .name("from")
                        .and_then(|x| Square::from_ascii(x.as_str().as_bytes()).ok())?,
                    to: cap
                        .name("to")
                        .and_then(|x| Square::from_ascii(x.as_str().as_bytes()).ok())?,
                    uncapture: cap
                        .name("uncapture")
                        .and_then(|x| x.as_str().chars().next())
                        .and_then(Role::from_char),
                    special_move: cap
                        .name("special_move")
                        .and_then(|x| SpecialMove::from_str(x.as_str()).ok()),
                })
            })
            .ok_or(ParseRetroUciError)
    }

    pub fn to_retro_uci(&self) -> String {
        format!(
            "{}{}{}{}",
            match self.special_move {
                Some(SpecialMove::UnPromotion) => "U".to_owned(),
                Some(SpecialMove::EnPassant) => "E".to_owned(),
                _ => "".to_owned(),
            },
            self.uncapture
                .map(|role| role.upper_char().to_string())
                .unwrap_or_else(|| "".to_owned()),
            self.from,
            self.to
        )
    }

    #[inline]
    pub fn is_unpromotion(&self) -> bool {
        self.special_move
            .map_or(false, |x| x == SpecialMove::UnPromotion)
    }

    #[inline]
    pub fn is_en_passant(&self) -> bool {
        self.special_move
            .map_or(false, |x| x == SpecialMove::EnPassant)
    }

    #[inline]
    pub fn mirror(&self) -> Self {
        Self {
            from: self.from.flip_vertical(),
            to: self.to.flip_vertical(),
            uncapture: self.uncapture,
            special_move: self.special_move,
        }
    }
}

impl fmt::Debug for UnMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_retro_uci())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_retro_uci_simple_move() {
        let simple_move: UnMove = UnMove::from_retro_uci("e2e4").unwrap();
        assert_eq!(simple_move.from, Square::E2);
        assert_eq!(simple_move.to, Square::E4);
        assert_eq!(simple_move.uncapture, None);
    }

    #[test]
    fn test_parse_retro_uci_uncapture() {
        let simple_move: UnMove = UnMove::from_retro_uci("Pe2e4").unwrap();
        assert_eq!(simple_move.from, Square::E2);
        assert_eq!(simple_move.to, Square::E4);
        assert_eq!(simple_move.uncapture.unwrap(), Role::Pawn);
    }

    #[test]
    fn test_parse_retro_uci_unpromotion() {
        let simple_move: UnMove = UnMove::from_retro_uci("Ue8e7").unwrap();
        assert_eq!(simple_move.from, Square::E8);
        assert_eq!(simple_move.to, Square::E7);
        assert!(simple_move.is_unpromotion());
    }

    #[test]
    fn test_parse_retro_uci_en_passant() {
        let simple_move: UnMove = UnMove::from_retro_uci("Ee3d4").unwrap();
        assert_eq!(simple_move.from, Square::E3);
        assert_eq!(simple_move.to, Square::D4);
        assert!(simple_move.is_en_passant());
    }

    #[test]
    fn test_to_uci() {
        for x in &["e2e4", "Pe2e4", "Ue8e7", "Ee3d4", "Qa1a2", "Ba1a2", "Nd4d5"] {
            let unmove: UnMove = UnMove::from_retro_uci(x).unwrap();
            assert_eq!(*x, &unmove.to_retro_uci());
            assert_eq!(format!("{:?}", unmove), *x);
        }
    }

    #[test]
    fn test_mirror() {
        assert_eq!(
            UnMove::from_retro_uci("a1a8").unwrap().mirror(),
            UnMove::from_retro_uci("a8a1").unwrap()
        );
        assert_eq!(
            UnMove::from_retro_uci("Qa1a8").unwrap().mirror(),
            UnMove::from_retro_uci("Qa8a1").unwrap()
        );
        assert_eq!(
            UnMove::from_retro_uci("Ua1a2").unwrap().mirror(),
            UnMove::from_retro_uci("Ua8a7").unwrap()
        );
        assert_eq!(
            UnMove::from_retro_uci("Ua1b2").unwrap().mirror(),
            UnMove::from_retro_uci("Ua8b7").unwrap()
        );
        assert_eq!(
            UnMove::from_retro_uci("Ef3e4").unwrap().mirror(),
            UnMove::from_retro_uci("Ef6e5").unwrap()
        );
    }
}
