
set -e
apt update
apt -y install gcc
curl https://sh.rustup.rs -sSf | sh -s - --default-toolchain stable -y
source ~/.cargo/env

curl -L https://github.com/thedodd/trunk/releases/latest/download/trunk-x86_64-unknown-linux-gnu.tar.gz --output trunk.tar.gz
tar -zxvf trunk.tar.gz

export PATH="$PATH:$PWD"

cargo install wasm-bindgen-cli

rustup target add wasm32-unknown-unknown
cargo update
trunk build --release
