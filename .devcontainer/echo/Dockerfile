# Debian bookworm dev echo backend (replaces third-party Alpine/scratch images).
FROM debian:bookworm-slim@sha256:60eac759739651111db372c07be67863818726f754804b8707c90979bda511df

RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /srv
RUN mkdir -p api && printf '{"status":"ok"}\n' > api/response.json

USER nobody
EXPOSE 3000
CMD ["python3", "-m", "http.server", "3000", "--bind", "0.0.0.0"]
