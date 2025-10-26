FROM debian:trixie

RUN apt-get update && apt-get install -y \
  curl build-essential pkg-config cmake libssl-dev libgl1-mesa-dev libglu1-mesa-dev \
  libxft-dev libxext-dev libxinerama-dev libxcursor-dev libxfixes-dev \
  libpango1.0-dev libglib2.0-dev libgio-2.0-dev \
  libcairo2-dev \
  && rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:$PATH"
