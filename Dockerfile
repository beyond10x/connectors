# syntax=docker/dockerfile:1.7
FROM rust:1.88-bookworm@sha256:af306cfa71d987911a781c37b59d7d67d934f49684058f96cf72079c3626bfe0 AS builder
WORKDIR /src
COPY . .
# The target directory is a persistent BuildKit cache. Git archives carry commit timestamps, so a
# cached artifact built later than a newer archive can otherwise look fresh to Cargo even when its
# source bytes changed. Refresh only workspace inputs (never the mounted target tree) before Cargo's
# freshness check so the image attests the code from SOURCE_SHA, not a stale local crate artifact.
RUN --mount=type=cache,id=b10x-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=b10x-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=b10x-connectors-target,target=/src/crates/connectors-cli/target,sharing=locked \
    find crates -path '*/target' -prune -o -type f -exec touch {} + && \
    cargo build --manifest-path crates/connectors-cli/Cargo.toml --locked --release && \
    cargo build --manifest-path crates/connectors-runtime/Cargo.toml --locked --release -p hosted-secrets --bin connectors-secrets-migrate && \
    install -D /src/crates/connectors-cli/target/release/connectors /out/connectors && \
    install -D /src/crates/connectors-runtime/target/release/connectors-secrets-migrate /out/connectors-secrets-migrate

FROM gcr.io/distroless/cc-debian12:nonroot@sha256:adcd20c7b4c988b73cbfbddb26d2eee574571e6d7c9ffea29b3821e0690efb77
ARG SOURCE_SHA=unknown
LABEL org.opencontainers.image.revision=$SOURCE_SHA
COPY --from=builder /out/connectors /usr/local/bin/connectors
COPY --from=builder /out/connectors-secrets-migrate /usr/local/bin/connectors-secrets-migrate
VOLUME ["/var/lib/b10x-connectors"]
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/connectors"]
CMD ["serve-hosted", "--config", "/etc/b10x-connectors/hosted.toml"]
