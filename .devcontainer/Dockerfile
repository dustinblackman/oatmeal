FROM rust:1.74.0-slim-bookworm as builder

RUN apt-get update && \
    apt-get install -y curl libssl-dev pkg-config

COPY ./Cargo.lock ./Cargo.toml ./.mise.toml ./rust-toolchain.toml /tmp/build-cache/
COPY .cargo/ /tmp/build-cache/.cargo

# Only install what we can fit within an arm64 container within the Github Actions time limit. The rest will be done at
# runtime. On amd64 all binaries will be installed.
WORKDIR /tmp/build-cache
RUN cargo install cargo-run-bin
RUN dpkg --print-architecture | grep -q 'arm64' && (cargo bin mise --help && cargo cmd --help) || cargo bin --install

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
    cargo cmd setup-nightly && \
    cargo bin mise install
