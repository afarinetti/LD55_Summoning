# Bomb the slime to survive!
Ludum Dare 55 Submission (Theme: Summoning)

## Dependencies

- Rust 1.77.2+
- `rustup target install wasm32-unknown-unknown`
- `cargo install wasm-server-runner`
- `cargo install trunk-ng`
- ...more... TBD

## Build/Run Instructions
### Desktop

- `cargo run`

### Web (Web Assembly)

- `cargo run --target wasm32-unknown-unknown`
or
- `trunk-ng serve --open`