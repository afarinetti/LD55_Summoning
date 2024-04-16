# Bomb the slime to survive!
Ludum Dare 55 Submission (Theme: Summoning)

<iframe frameborder="0" src="https://itch.io/embed/2649238" width="552" height="167"><a href="https://thulium.itch.io/bomb-the-slime-to-survive">Bomb the slime to survive! by thulium</a></iframe>

## Known Issues

1. Audio does not work on my Linux box, but works in WASM?!
2. Minions can spawn out of bounds
3. Player cannot die when not moved from initial corner -- physics sleeping?

## Dependencies

- Rust 1.77.2+
- See `Cargo.toml` for Rust crate dependencies.
- `rustup target install wasm32-unknown-unknown`
- `cargo install trunk-ng`
- or- `cargo install wasm-server-runner`

## Build/Run Instructions
### Desktop

- `cargo run`

### Web (Web Assembly)

- `cargo run --target wasm32-unknown-unknown`
- or- `trunk-ng build` and `trunk-ng serve --open`
- or- `trunk-ng build` and `npx serve dist` (requires npm and npx installed)
