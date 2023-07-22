FROM rust:1.71-slim-buster as builder

ARG TARGET=x86_64-unknown-linux-gnu

RUN apt-get update  \
    && apt-get upgrade -y  \
    && apt-get install -y libssl-dev libc-dev pkg-config  \
    && rustup target add ${TARGET}

WORKDIR build

COPY Cargo.toml Cargo.lock sqlx-data.json ./
COPY src ./src
COPY vendor ./vendor
COPY .cargo/config.toml .cargo/config.toml

RUN cargo build --release --offline --target ${TARGET} --jobs $(nproc) -vv

FROM ubuntu:20.04

ENV RUST_BACKTRACE full

LABEL maintainer="Kirill <k@kunansy.ru>"

RUN apt-get update  \
    && apt-get upgrade -y  \
    && apt-get install -y libssl-dev ca-certificates \
    && apt-get clean && apt-get autoclean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=umputun/cronn:latest /srv/cronn /srv/cronn
COPY --from=builder /build/target/release/app /app/app

COPY entrypoint.sh /app
RUN /app/entrypoint.sh \
    && rm entrypoint.sh

USER reminder
