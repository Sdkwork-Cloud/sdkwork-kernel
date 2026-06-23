# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder
WORKDIR /src
COPY . .
RUN cargo build -p sdkwork-agent-server --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --create-home --home-dir /var/lib/sdkwork --shell /usr/sbin/nologin sdkwork
WORKDIR /app
COPY --from=builder /src/target/release/sdkwork-agent-server /usr/local/bin/sdkwork-agent-server
RUN mkdir -p /var/lib/sdkwork/data \
    && chown -R sdkwork:sdkwork /var/lib/sdkwork
USER sdkwork
ENV SDKWORK_KERNEL_ENVIRONMENT=production \
    SDKWORK_KERNEL_HOSTING=cloud-hosted \
    SDKWORK_KERNEL_RUNTIME_TARGET=server \
    SDKWORK_BIND_ADDRESS=0.0.0.0 \
    SDKWORK_PORT=18280 \
    SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE=postgres \
    SDKWORK_KERNEL_INGRESS_AUTH_MODE=token \
    SDKWORK_KERNEL_METRICS_AUTH_MODE=token \
    SDKWORK_RATE_LIMIT_RPS=100 \
    SDKWORK_RATE_LIMIT_BURST=200
EXPOSE 18280
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD ["/usr/bin/curl", "-fsS", "http://127.0.0.1:18280/health"]
ENTRYPOINT ["/usr/local/bin/sdkwork-agent-server"]
