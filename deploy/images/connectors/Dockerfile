# syntax=docker/dockerfile:1.7
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:08476c244888bfb425bae1ae4dd7c17f093e972aab8426883d97f3c6f280f7c1
ARG TARGETARCH
ARG VERSION=unknown
ARG SOURCE_SHA=unknown
LABEL org.opencontainers.image.source="https://github.com/beyond10x/connectors" \
      org.opencontainers.image.title="connectors" \
      org.opencontainers.image.description="The Connectors release binaries; starts the catalog provider's served instance" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0" \
      org.opencontainers.image.version=$VERSION \
      org.opencontainers.image.revision=$SOURCE_SHA
COPY --chmod=0755 linux-${TARGETARCH}/ /usr/local/bin/
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/connectors-catalog-provider"]
