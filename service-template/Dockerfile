# --- Stage 1: Build Stage ---
# This stage compiles the Rust application and handles dependency caching.
FROM rust:latest as builder

# 1. Create a new, empty workspace member for our application.
# This allows us to leverage cargo-chef for efficient dependency caching.
WORKDIR /app
RUN cargo new --bin service

# 2. Copy over the manifests and lock file.
# This is a key change: We set the WORKDIR to the new crate's root,
# then copy our *actual* manifests into it, overwriting the template files.
WORKDIR /app/service
COPY ./Cargo.toml ./Cargo.lock ./

# 3. Install cargo-chef and calculate the dependency plan.
# This plan is cached by Docker. As long as Cargo.toml/Cargo.lock don't change,
# this layer will be reused, skipping the lengthy dependency compilation.
RUN cargo install cargo-chef && \
    cargo chef prepare --recipe-path ../recipe.json

# 4. Build the cached dependencies.
# This is the most time-consuming step and will be cached effectively.
RUN cargo chef cook --release --recipe-path ../recipe.json

# 5. Copy the application's source code and build it.
# This step is relatively fast as all dependencies are already compiled.
COPY ./src ./src
RUN cargo build --release --bin {{project-name}}-server


# --- Stage 2: Runtime Stage ---
# This stage creates a minimal final image for running the application.
FROM debian:stable-slim

# Set up a non-root user for security best practices.
RUN groupadd --system --gid 1001 appuser && \
    useradd --system --uid 1001 --gid 1001 appuser
USER appuser

# Copy the compiled binary from the builder stage to the final image.
COPY --from=builder /app/service/target/release/{{project-name}}-server /usr/local/bin/

# Expose the port the application will listen on.
# Defaulting to 3000, but can be overridden by the PORT env var at runtime.
EXPOSE 3000

# Set the entrypoint for the container.
# This is the command that will be run when the container starts.
CMD ["{{project-name}}-server"]
