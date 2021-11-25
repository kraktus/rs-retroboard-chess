use shakmaty::{Color, Color::Black, Color::White, Role};
use std::fmt;
use std::fmt::Write;
use std::str::FromStr;

// use arrayvec::ArrayVec;

/// Error when parsing an invalid retro UCI.
#[derive(Clone, Debug)]
pub struct ParseRetroPocketError;

/// A RetroBoard pocket with a counter for each piece type.
/// It stores the pieces than can be uncaptured by each color.    
/// `self.unpromotion` is the number of pieces than can unpromote into a pawn.
/// By default it is set to 0
#[derive(Eq, PartialEq, Clone, Hash)]
pub struct RetroPocket {
    pub pawn: u8,
    pub knight: u8,
    pub bishop: u8,
    pub rook: u8,
    pub queen: u8,
    pub unpromotion: u8,
}

impl RetroPocket {
    pub fn decr(&mut self, role: Role) {
        match role {
            Role::Pawn => self.pawn -= 1,
            Role::Knight => self.knight -= 1,
            Role::Bishop => self.bishop -= 1,
            Role::Rook => self.rook -= 1,
            Role::Queen => self.queen -= 1,
            _ => panic!("Cannot uncapture king"),
        }
    }
}

/// Empty pocket
impl Default for RetroPocket {
    fn default() -> Self {
        Self {
            pawn: 0,
            knight: 0,
            bishop: 0,
            rook: 0,
            queen: 0,
            unpromotion: 0,
        }
    }
}

impl fmt::Debug for RetroPocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('"')?;
        f.write_str(&"P".repeat(self.pawn as usize))?;
        f.write_str(&"N".repeat(self.knight as usize))?;
        f.write_str(&"B".repeat(self.bishop as usize))?;
        f.write_str(&"R".repeat(self.rook as usize))?;
        f.write_str(&"Q".repeat(self.queen as usize))?;
        if self.unpromotion > 0 {
            f.write_str(&self.unpromotion.to_string())?
        }
        f.write_char('"')
    }
}

impl FromStr for RetroPocket {
    type Err = ParseRetroPocketError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pawn: u8 = 0;
        let mut knight: u8 = 0;
        let mut bishop: u8 = 0;
        let mut rook: u8 = 0;
        let mut queen: u8 = 0;
        let mut unpromotion: Option<u8> = None;
        for c in s.chars() {
            if c.is_digit(10) {
                // unpromotion
                match unpromotion {
                    Some(_) => return Err(ParseRetroPocketError),
                    None => {
                        unpromotion = Some(
                            c.to_digit(10)
                                .expect("RetroPocket unpromotion number, checked digit before")
                                as u8,
                        )
                    }
                }
            } else {
                match c.to_ascii_uppercase() {
                    'P' => pawn += 1,
                    'N' => knight += 1,
                    'B' => bishop += 1,
                    'R' => rook += 1,
                    'Q' => queen += 1,
                    _ => return Err(ParseRetroPocketError),
                }
            }
        }
        Ok(RetroPocket {
            pawn,
            knight,
            bishop,
            rook,
            queen,
            unpromotion: unpromotion.unwrap_or(0),
        })
    }
}

impl IntoIterator for RetroPocket {
    type Item = Role;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let mut v: Vec<Role> = vec![];
        if self.pawn > 0 {
            v.push(Role::Pawn)
        };
        if self.knight > 0 {
            v.push(Role::Knight)
        };
        if self.bishop > 0 {
            v.push(Role::Bishop)
        };
        if self.rook > 0 {
            v.push(Role::Rook)
        };
        if self.queen > 0 {
            v.push(Role::Queen)
        };
        v.into_iter()
    }
}

/// Wrapper around [`RetroPocket`] that provide handful functions to access them by color
#[derive(Eq, PartialEq, Clone, Debug, Hash)]
pub struct RetroPockets {
    // TODO check if worth switching to `ByColor`
    black: RetroPocket,
    white: RetroPocket,
}

impl RetroPockets {
    pub fn color(&self, c: Color) -> &RetroPocket {
        match c {
            White => &self.white,
            Black => &self.black,
        }
    }
    pub fn color_mut(&mut self, c: Color) -> &mut RetroPocket {
        match c {
            White => &mut self.white,
            Black => &mut self.black,
        }
    }

    pub fn from_str(white: &str, black: &str) -> Result<Self, ParseRetroPocketError> {
        Ok(Self {
            white: RetroPocket::from_str(white)?,
            black: RetroPocket::from_str(black)?,
        })
    }
}

/// Empty pocket for each side
impl Default for RetroPockets {
    fn default() -> Self {
        Self {
            white: RetroPocket::default(),
            black: RetroPocket::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_pocket(
        p: RetroPocket,
        pawn: u8,
        knight: u8,
        bishop: u8,
        rook: u8,
        queen: u8,
        unpromotion: u8,
    ) {
        assert_eq!(p.pawn, pawn);
        assert_eq!(p.knight, knight);
        assert_eq!(p.bishop, bishop);
        assert_eq!(p.rook, rook);
        assert_eq!(p.queen, queen);
        assert_eq!(p.unpromotion, unpromotion);
    }

    #[test]
    fn test_retropocket_fromstr() {
        let r = RetroPocket::default();
        check_pocket(r, 0, 0, 0, 0, 0, 0);
        let r2 = RetroPocket::from_str("PNBRQ").unwrap();
        check_pocket(r2, 1, 1, 1, 1, 1, 0);
        for i in 1..10 {
            let r3 = RetroPocket::from_str(&("PNBRQ".to_owned() + &i.to_string())).unwrap();
            check_pocket(r3, 1, 1, 1, 1, 1, i);
        }
        assert!(RetroPocket::from_str("PNBRQ12").is_err());
    }

    #[test]
    fn test_retropocket_eq() {
        assert_eq!(
            RetroPocket::from_str("PQP").unwrap(),
            RetroPocket::from_str("PPQ").unwrap()
        );
        assert_eq!(RetroPocket::default(), RetroPocket::default());
        assert_ne!(
            RetroPocket::from_str("2NBRQ").unwrap(),
            RetroPocket::from_str("NBRQ6").unwrap()
        );
    }

    #[test]
    fn test_into_iter() {
        for conf in &["PNB", "BRQ", "PNBRQ"] {
            // need to be in the right order
            let r = RetroPocket::from_str(conf).unwrap();
            println!("{:?}", r.clone().into_iter());
            for (x, y) in conf
                .chars()
                .map(|c| Role::from_char(c).unwrap())
                .zip(r.into_iter())
            {
                assert_eq!(x, y)
            }
        }
    }
}
