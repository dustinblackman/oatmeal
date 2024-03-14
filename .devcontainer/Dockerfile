FROM rust:1.74.0-slim-bookworm as builder

RUN apt-get update && \
    apt-get install -y curl libssl-dev pkg-config

COPY ./Cargo.lock ./Cargo.toml ./.mise.toml /tmp/build-cache/
COPY .cargo/ /tmp/build-cache/.cargo

# Only install what is needed for general development.
WORKDIR /tmp/build-cache
COPY .bin/ /tmp/build-cache/
RUN ls -lsa /tmp/build-cache
RUN cargo install cargo-run-bin #bin
RUN cargo bin committed --help #bin
RUN cargo bin mise --help #bin
RUN cargo cmd --help #bin
RUN cargo nextest --help #bin
# RUN cargo insta --help
# RUN cargo deny --help
# RUN cargo watch --help

FROM rust:1.74.0-slim-bookworm

# TODO: Root for devpod, but I'd rather not... Fix later.
RUN groupadd --gid 1000 oatmeal && \ 
    useradd --uid 1000 --gid 1000 --shell /bin/bash --create-home oatmeal && \
    mkdir -p /etc/sudoers.d/ && \
    echo oatmeal ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/oatmeal && \
    chmod 0440 /etc/sudoers.d/oatmeal

USER oatmeal

COPY --chown=1000:1000 --from=builder /usr/local/cargo/bin/cargo-bin /usr/local/cargo/bin/cargo-bin
COPY --chown=1000:1000 --from=builder /tmp/build-cache/ /tmp/build-cache

RUN cd /tmp/build-cache/ && \
    cargo --help && \
    cargo cmd setup-nightly && \
    cargo bin mise install
