FROM rust:slim-bullseye

# Install bash + I/O monitoring tools + build essentials + python
RUN apt-get update && apt-get install -y \
    bash \
    vim \
    less \
    htop \
    iotop \
    dstat \
    sysstat \
    procps \
    lsof \
    strace \
    gcc \
    make \
    pkg-config \
    libssl-dev \
    python3 \
    python3-pip \
    python3-venv \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install uv (Python package/dependency manager)
RUN curl -LsSf https://astral.sh/uv/install.sh | sh && \
    mv /root/.cargo/bin/uv /usr/local/bin/uv

# Default workdir
WORKDIR /usr/app

# Copy Rust source code
COPY src ./src  
COPY Cargo.toml .
COPY Cargo.lock .

# Copy test files
COPY tests ./tests

# Default to interactive bash
CMD ["/bin/bash"]