FROM rust:1.69-alpine3.17 as builder

RUN set -e  \
    && apk upgrade \
    && apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconf libpq-dev

WORKDIR build

COPY Cargo.toml Cargo.lock sqlx-data.json /build/
COPY src /build/src

# TODO: vendor dependencies
RUN cargo build --release --bins -vv

FROM alpine:3.17

LABEL maintainer="Kirill <k@kunansy.ru>"

RUN set -e &&  \
    apk add libc6-compat

WORKDIR /app

COPY --from=umputun/cronn:latest /srv/cronn /srv/cronn
COPY --from=builder /build/target/release/app /app/app

COPY entrypoint.sh /app
RUN /app/entrypoint.sh \
    && rm entrypoint.sh

USER reminder
