rm:
    cd bin/desktop && cargo bundle --release --format osx
    open target/release/bundle/osx/pitpls.app

[positional-arguments]
d *args:
    cargo run --locked -p desktop -- "$@"

rw:
    cargo run --release --locked -p desktop
