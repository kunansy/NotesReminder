FROM rust:1.80-slim-bullseye AS builder

ARG TARGET=x86_64-unknown-linux-gnu

RUN apt-get update  \
    && apt-get upgrade -y  \
    && apt-get install -y libc-dev pkg-config  \
    && rustup target add ${TARGET}

WORKDIR build

COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY vendor ./vendor
COPY .sqlx ./.sqlx
COPY src ./src

RUN cargo build --release --offline --target ${TARGET} --jobs $(nproc) -vv

FROM ubuntu:20.04

ARG TARGET=x86_64-unknown-linux-gnu
ENV RUST_BACKTRACE full

LABEL maintainer="Kirill <k@kunansy.ru>"

RUN apt-get update  \
    && apt-get upgrade -y  \
    && apt-get install -y ca-certificates \
    && apt-get clean && apt-get autoclean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=umputun/cronn:latest /srv/cronn /srv/cronn
COPY --from=builder /build/target/${TARGET}/release/app /app/app

COPY entrypoint.sh /app
RUN /app/entrypoint.sh \
    && rm entrypoint.sh

USER reminder
