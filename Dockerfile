FROM rust:1.91-alpine3.22 AS builder

WORKDIR /build

RUN apk add build-base cmake pkgconfig openssl-dev openssl-libs-static

COPY Cargo.toml Cargo.lock proxy .
ENV OPENSSL_NO_VENDOR 1
RUN cargo build --release

FROM alpine:3.22

COPY --from=builder /build/target/release/pxls-proxy /app/pxls-proxy

CMD [ "/app/pxls-proxy" ]
