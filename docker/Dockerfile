# VoIP CRM Dioxus - Web Build Dockerfile
#
# Build stage for web target

FROM rust:1.85-slim AS builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install dioxus-cli
RUN cargo install dioxus-cli

# Install wasm target
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

# Copy project files
COPY Cargo.toml Cargo.lock* ./
COPY Dioxus.toml ./
COPY src ./src
COPY assets ./assets

# Build for web
RUN dx build --release --platform web

# Production stage
FROM nginx:alpine

# Copy built files
COPY --from=builder /app/dist /usr/share/nginx/html

# Copy nginx config
RUN echo 'server { \
    listen 80; \
    server_name localhost; \
    root /usr/share/nginx/html; \
    index index.html; \
    location / { \
        try_files $uri $uri/ /index.html; \
    } \
    location /api { \
        proxy_pass http://backend:8081; \
        proxy_http_version 1.1; \
        proxy_set_header Upgrade $http_upgrade; \
        proxy_set_header Connection "upgrade"; \
        proxy_set_header Host $host; \
        proxy_set_header X-Real-IP $remote_addr; \
    } \
}' > /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
