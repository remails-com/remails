FROM node:22-bookworm AS frontend-builder

WORKDIR /app

COPY frontend/package.json frontend/package-lock.json ./
RUN npm install

COPY frontend/ ./
RUN npm run build

FROM rust:1.85-bookworm AS rust-builder

WORKDIR /app
COPY --from=frontend-builder /app/dist/ /app/frontend/dist/
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY .sqlx .sqlx/

# Don't depend on live sqlx during build use cached .sqlx
RUN SQLX_OFFLINE=true cargo build --release --bin app

FROM debian:bookworm-slim AS final
RUN apt-get update && apt-get upgrade -y

# create a non root user to run the binary
ARG user=nonroot
ARG group=nonroot
ARG uid=2000
ARG gid=2000
RUN addgroup --gid ${gid} ${group} && adduser --uid ${uid} --gid ${gid} --system --disabled-login --disabled-password ${user}
EXPOSE 3000
# get the pre-built binary from rust-builder
COPY --from=rust-builder --chown=nonroot:nonroot /app/target/release/app /home/nonroot/app
RUN chmod 777 /home/nonroot/app

USER $user

ENTRYPOINT ["/home/nonroot/app"]
