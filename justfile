# Check the entire workspace
check:
    cargo check --workspace

# Build the zkVM guest program (via build.rs)
build-guest:
    cargo build -p example-proof-builder

# Build the escrow Wasm smart contract
build-escrow:
    cargo build -p escrow --release --target wasm32v1-none

# Build everything (guest + escrow)
build: build-guest build-escrow

# Run the CLI prover with two prime numbers (e.g. just prove 17 19)
prove *args:
    cargo run -p cli -- {{ args }}

# Run integration tests (requires a local rippled node)
test:
    RIPPLED_DOCKER_IMAGE=rippled:groth5-devnet cargo test

# Install the wasm32v1-none target
setup:
    rustup target add wasm32v1-none

# Build the rippled Docker image for testing
build-docker:
    docker build --platform=linux/amd64 -t rippled:groth5-devnet -f ./docker/rippled.Dockerfile .

# Run a standalone rippled node for testing
start-devnet:
    docker run \
      -p 5005:5005 \
      -p 6006:6006 \
      -p 51235:51235 \
      -v "./tests/rippled.cfg:/var/lib/rippled/rippled.cfg:ro" \
      rippled:groth5-devnet \
      --standalone \
      --conf /var/lib/rippled/rippled.cfg

# Clean all build artifacts
clean:
    cargo clean
