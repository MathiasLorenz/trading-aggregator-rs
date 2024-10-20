# Stage 1: Build the application
FROM rust:1.82-slim-bookworm AS builder

# Create a new empty shell project
WORKDIR /app
RUN USER=root cargo new --bin trading-results-rs
WORKDIR /app/trading-results-rs

# Copy over the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# This build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# Copy your source tree
COPY src ./src

# Copy .sqlx for queries to be available
COPY .sqlx ./.sqlx
# Make sure we use the query cache in .sqlx
ENV SQLX_OFFLINE=true

# Build the project again with the actual source code
RUN cargo build --release

# Stage 2: Create the final image
# Would like to try an alpine image at some point, should be much slimmer, might need static linking
FROM debian:bookworm-slim AS runner

# Set the working directory
WORKDIR /usr/local/bin

# Copy the build artifact from the builder stage
COPY --from=builder /app/trading-results-rs/target/release/trading-results-rs .

# Create empty dotenv file as one is needed to run the program
# Environment variables are passed when calling docker run instaed of this file
RUN touch .env

# Ensure the binary has execute permissions
RUN chmod +x trading-results-rs

# Set the startup command to run your binary
CMD ["./trading-results-rs"]
