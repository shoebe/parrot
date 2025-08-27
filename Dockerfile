# Build image
FROM rust:alpine as builder
RUN apk update && apk add wget build-base cmake pkgconf openssl-dev openssl-libs-static automake autoconf

COPY . .
RUN cargo install --path .

RUN wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -O /usr/local/bin/yt-dlp
RUN chmod a+rx /usr/local/bin/yt-dlp  # Make executable

# Release image
# Necessary dependencies to run Parrot
FROM alpine:latest

RUN apk update && apk add --no-cache python3

RUN mkdir -p /bin
COPY --from=builder /usr/local/bin/yt-dlp /usr/local/bin/yt-dlp 
RUN /usr/local/bin/yt-dlp  -U

COPY --from=builder /usr/local/cargo/bin/parrot /usr/local/bin/parrot

CMD ["parrot"]
