# Build image
# Necessary dependencies to build Parrot
FROM rust:slim-trixie as build

RUN apt-get update && apt-get install -y \
    build-essential autoconf automake cmake libtool libssl-dev pkg-config

WORKDIR "/parrot"

# Cache cargo build dependencies by creating a dummy source
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs
COPY Cargo.toml ./
COPY Cargo.lock ./
RUN cargo build --release --locked

COPY . .
RUN cargo build --release --locked

# Release image
# Necessary dependencies to run Parrot
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y python3 ffmpeg wget

RUN mkdir -p /bin
RUN wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -O /bin/yt-dlp
RUN chmod a+rx /bin/yt-dlp  # Make executable
RUN /bin/yt-dlp -U

COPY --from=build /parrot/target/release/parrot .

CMD ["./parrot"]
