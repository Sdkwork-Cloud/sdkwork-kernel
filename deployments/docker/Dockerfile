# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder
WORKDIR /src
COPY . .
RUN cargo build -p sdkwork-agent-server --release

FROM node:22-bookworm-slim AS node-runtime

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --create-home --home-dir /var/lib/sdkwork --shell /usr/sbin/nologin sdkwork
WORKDIR /app
COPY --from=builder /src/target/release/sdkwork-agent-server /usr/local/bin/sdkwork-agent-server
COPY --from=node-runtime /usr/local/bin/node /usr/local/bin/node
COPY --from=builder /src/scripts/provider-transport-workers/generic-ts-sdk-worker.mjs /app/provider-runtime/workers/generic-ts-sdk-worker.mjs
COPY --from=builder /src/scripts/provider-transport-workers/engine-sdk-live.mjs /app/provider-runtime/workers/engine-sdk-live.mjs
COPY --from=builder /src/scripts/provider-transport-workers/codex-cli-live.mjs /app/provider-runtime/workers/codex-cli-live.mjs
COPY --from=builder /src/scripts/provider-transport-workers/provider-cli-live.mjs /app/provider-runtime/workers/provider-cli-live.mjs
RUN mkdir -p /var/lib/sdkwork/data \
    && /usr/local/bin/node --version \
    && test -f /app/provider-runtime/workers/generic-ts-sdk-worker.mjs \
    && chown -R sdkwork:sdkwork /var/lib/sdkwork
USER sdkwork
ENV SDKWORK_KERNEL_ENVIRONMENT=production \
    SDKWORK_KERNEL_DEPLOYMENT_PROFILE=cloud \
    SDKWORK_KERNEL_RUNTIME_TARGET=server \
    SDKWORK_KERNEL_APPLICATION_PUBLIC_INGRESS_BIND=0.0.0.0:18280 \
    SDKWORK_KERNEL_AGENT_PLUGIN=rig \
    SDKWORK_AGENT_NODE_BINARY=/usr/local/bin/node \
    SDKWORK_AGENT_PROVIDER_RUNTIME_ROOT=/app/provider-runtime \
    SDKWORK_DATABASE_ENGINE=postgresql \
    SDKWORK_KERNEL_INGRESS_AUTH_MODE=token \
    SDKWORK_KERNEL_METRICS_AUTH_MODE=token \
    SDKWORK_RATE_LIMIT_RPS=100 \
    SDKWORK_RATE_LIMIT_BURST=200
EXPOSE 18280
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD ["/usr/bin/curl", "-fsS", "http://127.0.0.1:18280/healthz"]
ENTRYPOINT ["/usr/local/bin/sdkwork-agent-server"]
