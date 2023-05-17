FROM rust:1.69-slim-buster as builder

RUN apt-get update  \
    && apt-get upgrade -y  \
    && apt-get install -y libssl-dev libc-dev pkg-config

WORKDIR build

COPY Cargo.toml Cargo.lock sqlx-data.json /build/
COPY src /build/src
COPY vendor /build/vendor
COPY .cargo/config.toml .cargo/config.toml

# TODO: vendor dependencies
RUN cargo build --release --offline --bins -vv -j $(nproc)

FROM ubuntu:20.04

LABEL maintainer="Kirill <k@kunansy.ru>"

RUN apt-get update  \
    && apt-get upgrade -y  \
    && apt-get install -y libssl-dev ca-certificates

WORKDIR /app

COPY --from=umputun/cronn:latest /srv/cronn /srv/cronn
COPY --from=builder /build/target/release/app /app/app

COPY entrypoint.sh /app
RUN /app/entrypoint.sh \
    && rm entrypoint.sh

USER reminder
