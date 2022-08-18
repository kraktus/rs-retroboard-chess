#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
#![allow(clippy::semicolon_if_nothing_returned)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![doc = include_str!("../README.md")]

mod unmove;
pub use crate::unmove::{MoveKind, UnMove, UnMoveList};

mod retroboard;
pub use crate::retroboard::{perft, RetroBoard};

mod retropocket;
pub use crate::retropocket::{ParseRetroPocketError, RetroPocket, RetroPockets};

/// Using the re-exported version of shakmaty will ensure no version clashes
pub mod shakmaty {
    pub use shakmaty::*;
}
