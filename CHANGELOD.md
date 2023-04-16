# Changelog for retroboard

## To be added

## v0.2.10

- Update shakmaty to `v0.24`

## v0.2.9

- Fix generation of legal unmoves with two checkers when the closest check is also a blocker, for example in: `8/8/8/8/R7/2Q5/1B6/k1K5 b - -`, Ba3 is not legal because the bishop is a blocker of the queen.

## v0.2.8

- Fix generation of legal unmoves with two checkers (one stepper, one slider) when they are at the same distance from the king, example: `8/8/8/8/8/5k2/8/K3N2B b - - 0 1`

## v0.2.7

- Update shakmaty to `v0.23`

## v0.2.6

- Implement `From<Chess> for RetroBoard`

## v0.2.5

- Implement `fmt::Display` for `RetroBoard`
- Update shakmaty to `v0.22`

## v0.2.4

- Optimise `From<RetroBoard> for Chess` (-91% runtime)
- Update shakmaty to `v0.21.3`
- Re-export `shakmaty` to avoid version collision clash

## v0.2.3

- Update shakmaty to `v0.21.1`
- Use `shakmaty::relative_shift`

## v0.2.2

- Update shakmaty to `v0.21.0`
- Implement `Copy`, `fmt::Display` and `std::error::Error` for `ParseRetroPocketError`.

## v0.2.1

- Update shakmaty to `v0.20.3`
- Add `RetroBoard::flip_vertical`, `RetroBoard::flip_horizontal`, `RetroBoard::flip_diagonal`, `RetroBoard::flip_anti_diagonal`, `RetroBoard::rotate_90`, `RetroBoard::rotate_180`, `RetroBoard::rotate_270`, following their addition to `shakmaty::Board`.

## v0.2.0

- Add `RetroBoard::epd` to `Debug` output of `RetroBoard`
- `RetroBoard::new` and `RetroBoard::new_no_pockets` now return `Result<Self, ParseFenError>`
- Implement `From<ParseRetroPocketError>` for `ParseFenError`
- `RetroPocket::IntoIterator` now uses `ArrayVec` internally. \~60% speed up for the **whole** unmove generation

## v0.1.3

- Add `RetroBoard::retro_turn` method.

## v0.1.2

- Make `RetroBoard::king_of` method public.

## v0.1.1

- Implement `shakmaty::FromSetup` to `RetroBoard`.