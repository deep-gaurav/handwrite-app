
set -e

curl https://sh.rustup.rs -sSf | sh -s - --default-toolchain stable -y
source ~/.cargo/env

cargo install trunk

cargo install wasm-bindgen-cli

rustup target add wasm32-unknown-unknown
cargo update
trunk build --release
