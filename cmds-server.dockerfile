
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
RUN cargo build --profile=$PROFILE --bin all-cmds-server --bin deno-cmds-server --quiet
RUN cp /build/target/*/all-cmds-server /build/all-cmds-server
RUN cp /build/target/*/deno-cmds-server /build/deno-cmds-server

# Step 4:
# Create a tiny output image.
# It only contains our final binaries.
FROM docker.io/library/debian:stable-slim AS runtime
COPY ./certs/supabase-prod-ca-2021.crt /usr/local/share/ca-certificates/
RUN apt-get update && apt-get install -y libssl3 ca-certificates lld
WORKDIR /space-operator/
COPY --from=builder /build/all-cmds-server /usr/local/bin
COPY --from=builder /build/deno-cmds-server /usr/local/bin
RUN bash -c "ldd /usr/local/bin/* | (! grep 'not found')" && apt-get remove -y lld && rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["all-cmds-server"]
