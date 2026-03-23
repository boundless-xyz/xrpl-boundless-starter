# syntax=docker/dockerfile:1.6
ARG UBUNTU_VERSION=24.04

############################
# Build stage
############################
FROM --platform=linux/amd64 ubuntu:${UBUNTU_VERSION} AS build

ARG DEBIAN_FRONTEND=noninteractive
ARG BUILD_JOBS=2

# Pin what to build
ARG RIPPLED_REPO=https://github.com/XRPLF/rippled.git
ARG RIPPLED_REF=f615764e8a5eb100381d25fb97b10fe49a963128    # can be commit SHA, tag, or branch
ARG RIPPLED_DEPTH=1                                         # set to 0 for full clone if you need it

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl git \
    python3 python3-full python3-venv \
    cmake ninja-build \
    build-essential pkg-config \
    gcc g++ \
    libssl-dev zlib1g-dev \
  && rm -rf /var/lib/apt/lists/*

# Conan in venv (PEP 668-safe)
ENV VENV=/opt/conan-venv
RUN python3 -m venv "${VENV}" \
 && "${VENV}/bin/pip" install --no-cache-dir --upgrade pip setuptools wheel \
 && "${VENV}/bin/pip" install --no-cache-dir "conan>=2.17,<3"
ENV PATH="${VENV}/bin:${PATH}"

# Clone rippled source inside the image
WORKDIR /src
RUN --mount=type=cache,target=/root/.cache/git \
    if [ "${RIPPLED_DEPTH}" = "0" ]; then \
      git clone "${RIPPLED_REPO}" rippled; \
    else \
      git clone --depth "${RIPPLED_DEPTH}" "${RIPPLED_REPO}" rippled; \
    fi

WORKDIR /src/rippled
RUN git fetch --depth 1 origin "${RIPPLED_REF}" || true \
 && git checkout "${RIPPLED_REF}"

# Install provided profile + add XRPLF remote (patched recipes)
# Ensure default profile selects C++20 + libstdc++11 ABI as per BUILD.md tweaks.
RUN --mount=type=cache,target=/root/.conan2 \
    conan config install conan/profiles/ -tf "$(conan config home)/profiles/" \
 && conan remote add --index 0 --insecure xrplf https://conan.ripplex.io || true \
 && sed -i 's|^compiler\.cppstd=.*$|compiler.cppstd=20|' "$(conan config home)/profiles/default" \
 && sed -i 's|^compiler\.libcxx=.*$|compiler.libcxx=libstdc++11|' "$(conan config home)/profiles/default"

# Build & install deps in a build dir (BUILD.md)
RUN --mount=type=cache,target=/root/.conan2 \
    mkdir -p .build \
 && cd .build \
 && conan install .. --output-folder . --build missing --settings build_type=Release \
 && cmake \
      -G Ninja \
      -DCMAKE_TOOLCHAIN_FILE:FILEPATH=build/generators/conan_toolchain.cmake \
      -DCMAKE_BUILD_TYPE=Release \
      -Dxrpld=ON \
      -Dtests=OFF \
      .. \
 && cmake --build . -j "${BUILD_JOBS}"

############################
# Runtime stage
############################
FROM --platform=linux/amd64 ubuntu:${UBUNTU_VERSION} AS runtime
ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
  && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 10001 rippled
WORKDIR /var/lib/rippled

COPY --from=build /src/rippled/.build/rippled /usr/local/bin/rippled

ENV RIPPLED_DATA=/var/lib/rippled
RUN mkdir -p "$RIPPLED_DATA" && chown -R rippled:rippled "$RIPPLED_DATA"

USER rippled
EXPOSE 5005 6006 51235

ENTRYPOINT ["/usr/local/bin/rippled"]
CMD ["--help"]
