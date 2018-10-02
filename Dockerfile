FROM rust:slim AS build
WORKDIR /usr/src/blag
RUN apt-get update \
    && apt-get install -y musl-tools \
    && rustup target add x86_64-unknown-linux-musl
COPY . .
RUN cargo build --release --target=x86_64-unknown-linux-musl \
    && strip --strip-debug target/x86_64-unknown-linux-musl/release/blag

FROM alpine:latest
COPY --from=build /usr/src/blag/target/x86_64-unknown-linux-musl/release/blag /usr/local/bin/blag
ENTRYPOINT ["/usr/local/bin/blag"]
CMD ["--help"]
