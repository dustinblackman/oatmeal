FROM rust:1.74.0-slim-bookworm as builder

RUN apt-get update && \
    apt-get install -y curl libssl-dev pkg-config && \
    apt-get clean && \
    apt-get autoclean

# TODO Devpod is overriding the context directory and won't allow me to copy in files from outside the .devcontainer
# folder. Lazily download them from master instead.
RUN mkdir -p /tmp/build-cache/.cargo && \
    cd /tmp/build-cache && \
    echo "Cargo.toml Cargo.lock .mise.toml .cargo/config.toml" | sed 's/ /\n/g' | while read f; do \
      curl -L -o "/tmp/build-cache/$f" "https://raw.githubusercontent.com/dustinblackman/oatmeal/main/$f"; \
    done; \
    mkdir .bin

# Only install what is needed for general development.
WORKDIR /tmp/build-cache
RUN cargo install cargo-run-bin
RUN cargo cmd setup-nightly
RUN cargo bin committed --help
RUN cargo bin mise --help
RUN cargo deny --help
RUN cargo insta --help
RUN cargo nextest --help
RUN cargo watch --help

FROM rust:1.74.0-slim-bookworm

# TODO: Root for devpod, but I'd rather not... Fix later.
RUN groupadd --gid 1000 oatmeal && \ 
    useradd --uid 1000 --gid 1000 --shell /bin/bash --create-home oatmeal && \
    mkdir -p /etc/sudoers.d/ && \
    echo oatmeal ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/oatmeal && \
    chmod 0440 /etc/sudoers.d/oatmeal

USER oatmeal

COPY --chown=1000:1000 --from=builder /usr/local/cargo/bin/cargo-bin /usr/local/cargo/bin/cargo-bin
COPY --chown=1000:1000 --from=builder /tmp/build-cache/.bin /tmp/build-cache/Cargo.toml /tmp/build-cache/Cargo.lock /tmp/build-cache/.bin
RUN cd /tmp/build-cache/ && cargo --help

