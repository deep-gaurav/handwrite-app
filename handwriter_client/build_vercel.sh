
set -e
git clone https://89b81c9198c7975942f82cf05ecc040ded55051f@github.com/deep-gaurav/handwriter.git ../handwriter
curl https://sh.rustup.rs -sSf | sh -s - --default-toolchain stable -y
source ~/.cargo/env

cargo install trunk

cargo install wasm-bindgen-cli

rustup target add wasm32-unknown-unknown
cargo update
trunk build --release
