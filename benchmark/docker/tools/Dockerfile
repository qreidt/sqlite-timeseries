# Tools image: TSBS (data generator, query generator, TimescaleDB query runner),
# timescaledb-parallel-copy and psql.
#
# TSBS is pinned to a commit and built with an older Go, close to what its go.mod and CI expect
# (Go 1.15); timescaledb-parallel-copy needs a current Go. Override with --build-arg if needed.

ARG TSBS_GO_IMAGE=golang:1.17-bullseye
ARG PCOPY_GO_IMAGE=golang:1.24-bookworm

FROM ${TSBS_GO_IMAGE} AS tsbs
ARG TSBS_REPO=https://github.com/timescale/tsbs.git
ARG TSBS_COMMIT=8323e59c74027b108f4ad5ec5d3e498b0101a02e
RUN git clone "$TSBS_REPO" /src/tsbs && git -C /src/tsbs checkout --quiet "$TSBS_COMMIT"
WORKDIR /src/tsbs
RUN mkdir -p /out && CGO_ENABLED=0 go build -o /out/ \
      ./cmd/tsbs_generate_data \
      ./cmd/tsbs_generate_queries \
      ./cmd/tsbs_run_queries_timescaledb

FROM ${PCOPY_GO_IMAGE} AS pcopy
ARG PARALLEL_COPY_VERSION=latest
RUN CGO_ENABLED=0 GOBIN=/out go install \
      github.com/timescale/timescaledb-parallel-copy/cmd/timescaledb-parallel-copy@${PARALLEL_COPY_VERSION}

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends postgresql-client gzip ca-certificates \
 && rm -rf /var/lib/apt/lists/*
COPY --from=tsbs /out/ /usr/local/bin/
COPY --from=pcopy /out/ /usr/local/bin/
ARG TSBS_COMMIT=8323e59c74027b108f4ad5ec5d3e498b0101a02e
ENV TSBS_COMMIT=${TSBS_COMMIT}
WORKDIR /tmp
