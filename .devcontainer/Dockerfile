FROM rust:1.95.0-slim-bookworm

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LANGUAGE=en_US:en \
    LC_ALL=C.UTF-8 \
    RUSTUP_PERMIT_COPY_RENAME=true

RUN apt update -y && \
    apt install -y curl git sudo zstd make ripgrep


ARG CONTAINER_USERNAME=builder
ARG CONTAINER_USERID=1000

RUN useradd -u ${CONTAINER_USERID} -s /bin/bash -m ${CONTAINER_USERNAME} && \
    install --owner=${CONTAINER_USERNAME} --directory /work && \
    echo "${CONTAINER_USERNAME} ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/container && \
    chmod 440 /etc/sudoers.d/container && \
    \
    apt clean && \
    rm -rf /var/log/* ; \
    rm -rf /tmp/*

USER ${CONTAINER_USERNAME}

RUN curl -fsSL https://raw.githubusercontent.com/pirakansa/vorbere/main/install.sh | bash && \
    rustup default 1.95.0 && \
    rustup component add rustfmt clippy && \
    cargo install cargo-audit

WORKDIR /work
