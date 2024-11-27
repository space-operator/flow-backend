FROM docker.io/library/rust as rustc
RUN apt-get update && apt-get install -y clang lld && rm -rf /var/lib/apt/lists/*
ENV RUSTFLAGS="-C linker=clang -C link-arg=-fuse-ld=lld"
RUN cargo install cargo-chef --quiet
WORKDIR /build/

# Step 1: Compute a recipe file
FROM rustc as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Step 2: Cache project dependencies
FROM rustc as cacher
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Step 3: Build the binary
FROM rustc as builder
COPY . .
# Copy over the cached dependencies from above
COPY --from=cacher /build/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN cargo build --release --bin flow-server --quiet

FROM docker.io/denoland/deno:debian as deno

# Step 4:
# Create a tiny output image.
# It only contains our final binaries.
FROM docker.io/library/debian:stable-slim as runtime
COPY ./certs/supabase-prod-ca-2021.crt /usr/local/share/ca-certificates/
COPY ./crates/flow-server/entrypoint.bash /space-operator/
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates wget && \
    wget https://github.com/supabase/cli/releases/download/v1.167.4/supabase_1.167.4_linux_amd64.deb && \
    apt-get install -y ./supabase_1.167.4_linux_amd64.deb && \
    apt-get remove -y wget && \
    rm -rf /var/lib/apt/lists/*
COPY --from=deno /usr/bin/deno /usr/local/bin
RUN deno --version
WORKDIR /space-operator/
COPY --from=builder /build/target/release/flow-server /usr/local/bin
RUN bash -c "ldd /usr/local/bin/* | (! grep 'not found')"
ENTRYPOINT ["./entrypoint.bash"]
