[positional-arguments]
d *args:
    cargo run --locked -p desktop -- "$@"

r:
    cd bin/desktop && cargo bundle --release --format osx
    open target/release/bundle/osx/pitpls.app
