FROM tensorflow/tensorflow:1.6.0

ARG DEBIAN_FRONTEND=noninteractive

RUN apt update
RUN apt install -y python-tk git
RUN apt install -y curl build-essential aria2 unrar unzip tree zip wget wget

RUN cd / && git clone https://deep-gaurav:Rave22@deeP@github.com/deep-gaurav/handwriter.git

RUN cd /handwriter && pip install -r requirements.txt

RUN curl https://sh.rustup.rs -sSf --output rustinstaller
RUN sh rustinstaller -y
RUN export PATH="$PATH:$HOME/.cargo/bin" && cd /src && cargo build --release

CMD cd /src && ./target/release/heroku_handwriter
