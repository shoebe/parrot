# Build image
FROM rust:alpine AS builder
RUN apk update && apk add wget build-base cmake pkgconf openssl-dev openssl-libs-static automake autoconf

WORKDIR /app
COPY . .

# Cache cargo dependencies and build artifacts across builds
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo install --path .

RUN wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -O /usr/local/bin/yt-dlp
RUN chmod a+rx /usr/local/bin/yt-dlp  # Make executable

# Release image
# Necessary dependencies to run Parrot
FROM alpine:latest

RUN apk update && apk add --no-cache python3 ca-certificates

RUN mkdir -p /bin
COPY --from=builder /usr/local/bin/yt-dlp /usr/local/bin/yt-dlp 

# Schedule daily yt-dlp update via Alpine's periodic cron
RUN printf '#!/bin/sh\nyt-dlp -U || true\n' > /etc/periodic/daily/ytdlp-update && \
    chmod +x /etc/periodic/daily/ytdlp-update

COPY --from=builder /usr/local/cargo/bin/parrot /usr/local/bin/parrot
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["parrot"]
