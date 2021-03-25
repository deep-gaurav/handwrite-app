FROM tensorflow/tensorflow

ARG DEBIAN_FRONTEND=noninteractive

RUN apt update
RUN apt install -y python-tk git pkg-config libssl-dev
RUN apt install -y curl build-essential aria2 unrar unzip tree zip wget wget libcairo2-dev pkg-config python-dev python3-dev


RUN curl https://sh.rustup.rs -sSf --output rustinstaller
RUN sh rustinstaller -y

ADD . /src
RUN cd /src && git clone https://89b81c9198c7975942f82cf05ecc040ded55051f@github.com/deep-gaurav/handwriter.git
RUN cd /src/handwriter/ && git pull && cd .. && pip install -r requirements.txt


RUN export PATH="$PATH:$HOME/.cargo/bin" && cd /src && cargo build --release
CMD cd /src/handwriter/ && git pull && cd .. && RUST_LOG=info ./target/release/handwriter_server
