et:
    cd bin/desktop/src-tauri && cargo test --lib export_bindings

d:
    cd bin/desktop && npm run tauri dev

[positional-arguments]
dg *args:
    cargo run --locked -p desktop-gpui -- "$@"
