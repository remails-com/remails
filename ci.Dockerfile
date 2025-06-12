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

FROM final-base AS mta
ARG version=dev

COPY --chown=nonroot:nonroot ./target/release/mta ./mta
RUN chmod 777 mta

EXPOSE 3025
ENV VERSION=${version}
ENTRYPOINT ["./mta"]

FROM final-base AS retry
ARG version=dev

COPY --chown=nonroot:nonroot ./target/release/retry ./retry
RUN chmod 777 retry

ENV VERSION=${version}
ENTRYPOINT ["./retry"]

FROM final-base AS migrate-db
ARG version=dev

COPY --chown=nonroot:nonroot ./target/release/migrate_db ./migrate_db
RUN chmod 777 migrate_db

ENV VERSION=${version}
ENTRYPOINT ["./migrate_db"]