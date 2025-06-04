FROM rust:1.87-bookworm AS rust-builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY build.rs build.rs ./
COPY src/ src/
COPY .sqlx .sqlx/

RUN mkdir -p "/app/frontend/dist/"
# Don't depend on live sqlx during build use cached .sqlx
RUN SQLX_OFFLINE=true cargo build --release --bin mta

FROM debian:bookworm-slim AS final
RUN apt-get update && apt-get install libssl3 -y && apt-get upgrade -y

# create a non root user to run the binary
ARG user=nonroot
ARG group=nonroot
ARG uid=2000
ARG gid=2000
RUN addgroup --gid ${gid} ${group} && adduser --uid ${uid} --gid ${gid} --system --disabled-login --disabled-password ${user}
EXPOSE 3000
WORKDIR /home/nonroot
# get the pre-built binary from rust-builder
COPY --from=rust-builder --chown=nonroot:nonroot /app/target/release/mta ./mta
RUN chmod 777 mta

USER $user

ENTRYPOINT ["./mta"]
