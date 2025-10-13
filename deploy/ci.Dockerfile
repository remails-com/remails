FROM ubuntu:24.04 AS final-base
RUN apt-get update && apt-get install libssl3 adduser -y && apt-get upgrade -y

# create a non root user to run the binary
ARG user=nonroot
ARG group=nonroot
ARG uid=2000
ARG gid=2000
RUN addgroup --gid ${gid} ${group} && adduser --uid ${uid} --gid ${gid} --system --disabled-login --disabled-password ${user}

WORKDIR /home/nonroot
USER $user

FROM final-base AS management
ARG version=dev

COPY --chown=nonroot:nonroot ./target/release/management ./management
RUN chmod 777 management

EXPOSE 3000
ENV VERSION=${version}
ENTRYPOINT ["./management"]

FROM final-base AS inbound
ARG version=dev

COPY --chown=nonroot:nonroot ./target/release/inbound ./inbound
RUN chmod 777 inbound

EXPOSE 3025
ENV VERSION=${version}
ENTRYPOINT ["./inbound"]

FROM final-base AS outbound
ARG version=dev

COPY --chown=nonroot:nonroot ./target/release/outbound ./outbound
RUN chmod 777 outbound

EXPOSE 3025
ENV VERSION=${version}
ENTRYPOINT ["./outbound"]

FROM final-base AS periodic
ARG version=dev

COPY --chown=nonroot:nonroot ./target/release/periodic ./periodic
RUN chmod 777 periodic

ENV VERSION=${version}
ENTRYPOINT ["./periodic"]

FROM final-base AS migrate-db
ARG version=dev

COPY --chown=nonroot:nonroot ./target/release/migrate_db ./migrate_db
RUN chmod 777 migrate_db

ENV VERSION=${version}
ENTRYPOINT ["./migrate_db"]

FROM final-base AS message-bus
ARG version=dev

COPY --chown=nonroot:nonroot ./target/release/message_bus ./message_bus
RUN chmod 777 message_bus

ENV VERSION=${version}
ENTRYPOINT ["./message_bus"]