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
COPY vendor vendor
ARG PROFILE=release
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --profile=$PROFILE --recipe-path recipe.json

# Step 3: Build the binary
FROM rustc AS builder
COPY . .
# Copy over the cached dependencies from above
COPY --from=cacher /build/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
ARG PROFILE=release
RUN cargo build --profile=$PROFILE --bin flow-server --quiet
RUN cp /build/target/*/flow-server /build/flow-server

FROM docker.io/denoland/deno:debian AS deno

# Step 4:
# Create a tiny output image.
# It only contains our final binaries.
FROM docker.io/library/debian:stable-slim AS runtime
COPY ./certs/supabase-prod-ca-2021.crt /usr/local/share/ca-certificates/
COPY ./crates/flow-server/entrypoint.bash /space-operator/
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates wget lld && \
    wget https://github.com/supabase/cli/releases/download/v1.167.4/supabase_1.167.4_linux_amd64.deb && \
    apt-get install -y ./supabase_1.167.4_linux_amd64.deb && \
    apt-get remove -y wget
COPY --from=deno /usr/bin/deno /usr/local/bin
RUN deno --version
WORKDIR /space-operator/
COPY --from=builder /build/flow-server /usr/local/bin
RUN bash -c "ldd /usr/local/bin/* | (! grep 'not found')" && apt-get remove -y lld && rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["./entrypoint.bash"]
