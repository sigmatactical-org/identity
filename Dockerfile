## Build identity binary
FROM rust:1.90.0-alpine3.22 AS builder

RUN apk add --no-cache musl-dev nodejs npm git

WORKDIR /usr/src
RUN git clone --depth 1 https://github.com/sigmatactical-org/sigma-theme.git sigma-theme \
    && cd sigma-theme/ts && npm ci && npm run build

COPY . identity/
RUN cd identity/ts && npm ci && npm run build

WORKDIR /usr/src/identity
RUN mkdir -p .cargo && printf '%s\n' \
    '[patch."https://github.com/sigmatactical-org/sigma-theme.git"]' \
    'sigma-theme = { path = "../sigma-theme" }' \
    > .cargo/config.toml

RUN rustup target add x86_64-unknown-linux-musl \
    && update-ca-certificates \
    && cargo build --target x86_64-unknown-linux-musl --release \
    && strip -s target/x86_64-unknown-linux-musl/release/identity

## Final image
FROM alpine:3.22 AS runtime
ENV MIMALLOC_LARGE_OS_PAGES=1
COPY --from=builder /usr/src/identity/target/x86_64-unknown-linux-musl/release/identity /
COPY --from=builder /usr/src/identity/files /files
EXPOSE 3000
VOLUME ["/files"]
USER 65534
CMD ["/identity"]
