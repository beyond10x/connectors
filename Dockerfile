# syntax=docker/dockerfile:1.7
FROM rust:1.88-bookworm AS builder
WORKDIR /src
COPY . .
RUN cargo build --manifest-path crates/connectors-cli/Cargo.toml --locked --release

FROM gcr.io/distroless/cc-debian12:nonroot
ARG SOURCE_SHA=unknown
LABEL org.opencontainers.image.revision=$SOURCE_SHA
COPY --from=builder /src/crates/connectors-cli/target/release/connectors /usr/local/bin/connectors
VOLUME ["/var/lib/b10x-connectors"]
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/connectors"]
CMD ["serve-hosted", "--config", "/etc/b10x-connectors/hosted.toml"]
