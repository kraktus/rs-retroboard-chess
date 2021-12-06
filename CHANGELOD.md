# Changelog for retroboard


## v0.2.0

- `RetroBoard::new` and `RetroBoard::new_no_pockets` now return `Result<Self, ParseFenError>`
- Implement `From<ParseRetroPocketError>` for `ParseFenError`
- `RetroPocket::IntoIterator` now uses `ArrayVec` internally. ~60% speed up for the **whole** unmove generation

## v0.1.3

- Add `RetroBoard::retro_turn` method.

## v0.1.2

- Make `RetroBoard::king_of` method public.

## v0.1.1

- Implement `shakmaty::FromSetup` to `RetroBoard`.