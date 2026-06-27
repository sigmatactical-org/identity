# syntax=docker/dockerfile:1.6
# Runtime-only image. Build context must be the directory produced by
# `scripts/prepare-image-context.sh` (contains `identity` binary + `files/`).
# Distroless `base` is smaller but lacks libgcc_s.so.1, which this glibc-linked
# Rust binary needs; `cc` is the smallest stock distroless image that fits.
FROM gcr.io/distroless/cc-debian13:nonroot
WORKDIR /app

COPY --chmod=755 identity /app/identity
COPY --chown=nonroot:nonroot files /files

USER nonroot:nonroot

ENV MIMALLOC_LARGE_OS_PAGES=1
ENV IDENTITY_BIND_PORT=3000
EXPOSE 3000

ENTRYPOINT ["/app/identity"]
