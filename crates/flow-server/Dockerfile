FROM docker.io/library/rust AS rustc
RUN apt-get update && apt-get install -y capnproto && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --quiet
WORKDIR /build/

# Step 1: Compute a recipe file
FROM rustc AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


# Step 2: Cache project dependencies
FROM rustc AS cacher
ARG PROFILE=release
COPY vendor vendor
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --profile=$PROFILE --recipe-path recipe.json

# Step 3: Build the binary
FROM rustc AS builder
ARG PROFILE=release
COPY . .
# Copy over the cached dependencies from above
COPY --from=cacher /build/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN cargo build --profile=$PROFILE --bin flow-server --bin umbra-prover --quiet
RUN cp /build/target/*/flow-server /build/flow-server && \
    cp /build/target/*/umbra-prover /build/umbra-prover

FROM docker.io/denoland/deno:2.6.4 AS deno

# Step 3b: Install Bun workspace node_modules for native Bun command nodes.
# cmds-bun's spawn_running_bun() symlinks a workspace node_modules next to
# each tempdir, so the runtime image must ship both node_modules and the
# workspace member source dirs that node_modules/@space-operator/* point at.
FROM docker.io/oven/bun:1.2.3 AS bun-deps
WORKDIR /workspace
COPY package.json bun.lock ./
COPY @space-operator/bun-command-rpc/package.json ./@space-operator/bun-command-rpc/
COPY @space-operator/flow-lib-bun/package.json ./@space-operator/flow-lib-bun/
COPY crates/cmds-bun/package.json ./crates/cmds-bun/
COPY tools/loadtest/package.json ./tools/loadtest/
RUN bun install --frozen-lockfile
COPY @space-operator/bun-command-rpc ./@space-operator/bun-command-rpc
COPY @space-operator/flow-lib-bun ./@space-operator/flow-lib-bun

# Step 4:
# Create a tiny output image.
# It only contains our final binaries.
FROM docker.io/library/debian:stable-slim AS runtime
COPY ./certs/supabase-prod-ca-2021.crt /usr/local/share/ca-certificates/
COPY ./crates/flow-server/entrypoint.bash /space-operator/
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates wget lld libstdc++6 && \
    wget https://github.com/supabase/cli/releases/download/v1.167.4/supabase_1.167.4_linux_amd64.deb && \
    apt-get install -y ./supabase_1.167.4_linux_amd64.deb && \
    apt-get remove -y wget
COPY --from=deno /usr/bin/deno /usr/local/bin
RUN deno --version
COPY --from=bun-deps /usr/local/bin/bun /usr/local/bin/bun
RUN bun --version
WORKDIR /space-operator/
COPY --from=builder /build/flow-server /usr/local/bin
COPY --from=builder /build/umbra-prover /usr/local/bin
COPY --from=bun-deps /workspace/node_modules /space-operator/node_modules
COPY --from=bun-deps /workspace/@space-operator /space-operator/@space-operator
RUN bash -c "ldd /usr/local/bin/flow-server /usr/local/bin/umbra-prover /usr/local/bin/deno /usr/local/bin/bun | (! grep 'not found')" && apt-get remove -y lld && rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["./entrypoint.bash"]
