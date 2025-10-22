FROM node:22-alpine

# Install Rust nightly and required tools
RUN apk add --no-cache \
    build-base \
    curl \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && source ~/.cargo/env \
    && rustup default nightly-2025-10-20 \
    && rustup toolchain install nightly-2025-10-20 \
    && rustup target add wasm32-unknown-unknown --toolchain nightly-2025-10-20 \
    && rustup component add rust-src --toolchain nightly-2025-10-20 \
    && curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Set environment variables for Rust
ENV PATH="/root/.cargo/bin:$PATH"
ENV RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals"

# Set working directory
WORKDIR /app

# Copy package files
COPY package.json yarn.lock ./

# Install Node.js dependencies
RUN yarn install

# Copy source code
COPY . .

# Build the project
RUN yarn run build

# Expose port for serving
EXPOSE 8080

# Command to serve the built application
CMD ["yarn", "start", "--", "--host"]
