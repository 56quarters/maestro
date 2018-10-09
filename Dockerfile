FROM rust:slim AS build
WORKDIR /usr/src
RUN apt-get update \
    && apt-get install -y musl-tools \
    && rustup target add x86_64-unknown-linux-musl
COPY . .
RUN cargo build --release --target=x86_64-unknown-linux-musl \
    && strip --strip-debug target/x86_64-unknown-linux-musl/release/maestro

FROM alpine:latest
COPY --from=build /usr/src/target/x86_64-unknown-linux-musl/release/maestro /usr/local/bin/maestro
ENTRYPOINT ["/usr/local/bin/maestro"]
CMD ["--help"]
