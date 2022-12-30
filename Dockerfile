FROM rust as build

# create a new empty shell project
USER root

RUN \
    cargo new --bin cache-project

WORKDIR /cache-project

# Create dummy file to force cargo to build dependencies
RUN \
    touch src/lib.rs 

# Copy over manifests
COPY Cargo.lock Cargo.lock
COPY Cargo.toml Cargo.toml

# Build dependencies
RUN \
    cargo build \
        --release \
        --locked


# Remove and copy over source code
RUN rm src/*.rs

COPY src src
COPY migrations migrations
COPY VERSION VERSION

# Cleanup and build release image
RUN \
    rm target/release/log-ingest-api* && \
    rm target/release/deps/log_ingest_api*

RUN \
    cargo build \
        --release \
        --locked


FROM ubuntu:kinetic as production


RUN apt-get update && apt-get install -y \
    ca-certificates && \
    useradd -rm -d /app app

USER app
WORKDIR /usr/src/app

COPY --from=build --chown=app:app /cache-project/target/release/log-ingest-api /usr/local/bin/log-ingest-api

RUN \
    chmod +x /usr/local/bin/log-ingest-api

CMD ["log-ingest-api"]
