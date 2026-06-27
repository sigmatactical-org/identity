# syntax=docker/dockerfile:1.6
# COPY-only runtime image. CI builds the binary and stages build/image/ in GitHub Actions;
# this file only copies those artifacts into gcr.io/distroless/cc-debian13:nonroot.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:d3cda6e91129130d7229a1806b6a73d292ef245ab032da7851907798024cefba

WORKDIR /app

COPY --chmod=555 sigma-identity /app/sigma-identity
COPY --chown=nonroot:nonroot files /files

USER nonroot:nonroot

ENV MIMALLOC_LARGE_OS_PAGES=1
ENV PORT=3000
EXPOSE 3000

ENTRYPOINT ["/app/sigma-identity"]
