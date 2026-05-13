FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl wget file gnupg2 \
        build-essential pkg-config git \
        libssl-dev libgtk-3-dev librsvg2-dev \
        libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev \
        libayatana-appindicator3-dev \
        xdg-utils desktop-file-utils \
        xvfb \
    && rm -rf /var/lib/apt/lists/*

# Node 20 via NodeSource
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/* \
    && node --version && npm --version

# Rust stable via rustup
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --default-toolchain stable --profile minimal \
    && rustup component add rustfmt clippy \
    && rustup --version && rustc --version && cargo --version

# Make /app the canonical project location; bind-mounted at runtime
WORKDIR /app

# Cache hint: Tauri's dev server listens on 1420 by default
EXPOSE 1420

CMD ["bash"]
