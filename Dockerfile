FROM rust:slim AS builder

COPY . /ontime

WORKDIR /ontime

ARG TARGET="x86_64-unknown-linux-musl"

RUN apt update \
    && apt install -y musl-tools \
    && rustup target add "$TARGET" \
    && cargo build --release --target "$TARGET" \
    && strip target/${TARGET}/release/ontime


FROM bash:5.0

ARG TARGET="x86_64-unknown-linux-musl"
COPY --from=builder /ontime/target/${TARGET}/release/ontime /bin/

RUN ontime --version

ENTRYPOINT [ "/bin/bash", "-l", "-c" ]
