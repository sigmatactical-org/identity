# Debian bookworm dev echo backend (Rust; no Python).
FROM rust:1.90-bookworm AS build
WORKDIR /src
COPY echo/ ./echo/
RUN cd echo && cargo build --release

FROM debian:bookworm-slim@sha256:60eac759739651111db372c07be67863818726f754804b8707c90979bda511df
COPY --from=build /src/echo/target/release/sigma-identity-echo /usr/local/bin/sigma-identity-echo
USER nobody
EXPOSE 3000
CMD ["sigma-identity-echo"]
