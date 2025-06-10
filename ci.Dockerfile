#FROM node:22-bookworm AS frontend-builder
#
#WORKDIR /app
#
#COPY frontend/package.json frontend/package-lock.json ./
#RUN npm install
#
#COPY frontend/ ./
#RUN npm run build
#
#FROM rust:1.87-bookworm AS rust-builder
#
#WORKDIR /app
#COPY --from=frontend-builder /app/dist/ /app/frontend/dist/
#COPY Cargo.toml Cargo.lock ./
#COPY build.rs build.rs ./
#COPY src/ src/
#COPY .sqlx .sqlx/
#
## Don't depend on live sqlx during build use cached .sqlx
#RUN SQLX_OFFLINE=true cargo build --release --bins

FROM debian:bookworm-slim AS final-base
RUN apt-get update && apt-get install libssl3 -y && apt-get upgrade -y

# create a non root user to run the binary
ARG user=nonroot
ARG group=nonroot
ARG uid=2000
ARG gid=2000
RUN addgroup --gid ${gid} ${group} && adduser --uid ${uid} --gid ${gid} --system --disabled-login --disabled-password ${user}

WORKDIR /home/nonroot
USER $user

FROM final-base AS management

# get the pre-built binary from rust-builder
#COPY --from=rust-builder --chown=nonroot:nonroot /app/target/release/management ./management
COPY --chown=nonroot:nonroot ./target/release/management ./management
RUN chmod 777 management

EXPOSE 3000
ENTRYPOINT ["./management"]

FROM final-base AS mta

# get the pre-built binary from rust-builder
#COPY --from=rust-builder --chown=nonroot:nonroot /app/target/release/mta ./mta
COPY --chown=nonroot:nonroot ./target/release/mta ./mta
RUN chmod 777 mta

EXPOSE 3025
ENTRYPOINT ["./mta"]

FROM final-base AS retry

# get the pre-built binary from rust-builder
#COPY --from=rust-builder --chown=nonroot:nonroot /app/target/release/retry ./retry
COPY --chown=nonroot:nonroot ./target/release/retry ./retry
RUN chmod 777 retry

ENTRYPOINT ["./retry"]

FROM final-base AS migrate-db

# get the pre-built binary from rust-builder
#COPY --from=rust-builder --chown=nonroot:nonroot /app/target/release/migrate_db ./migrate_db
COPY --chown=nonroot:nonroot ./target/release/migrate_db ./migrate_db
RUN chmod 777 migrate_db

ENTRYPOINT ["./migrate_db"]