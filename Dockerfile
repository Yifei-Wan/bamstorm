FROM rust:slim-bullseye

# Install bash + I/O monitoring tools + build essentials
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
    && rm -rf /var/lib/apt/lists/*

# Copy your source code (optional, for building inside container)
WORKDIR /usr/app

COPY src ./src  
COPY Cargo.toml .
COPY Cargo.lock .

# Default to bash (interactive)
CMD ["/bin/bash"]
