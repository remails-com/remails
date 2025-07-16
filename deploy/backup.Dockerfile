FROM ubuntu:24.04

# Install postgresql client
ENV VERSION=dev
ENV POSTGRESQL_VERSION=16
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get upgrade -y \
    && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y \
    curl \
    ca-certificates \
    openssl \
    bzip2

RUN install -d /usr/share/postgresql-common/pgdg \
    && curl -o /usr/share/postgresql-common/pgdg/apt.postgresql.org.asc --fail https://www.postgresql.org/media/keys/ACCC4CF8.asc \
    && echo "deb [signed-by=/usr/share/postgresql-common/pgdg/apt.postgresql.org.asc] http://apt.postgresql.org/pub/repos/apt/ noble-pgdg main" > /etc/apt/sources.list.d/pgdg.list \
    && apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        postgresql-client-$POSTGRESQL_VERSION \
    && rm -rf /var/lib/apt/lists/*

# https://github.com/restic/restic/releases
ENV RESTIC_VERSION=0.18.0
# install restic, see https://restic.readthedocs.io/en/stable/020_installation.html#official-binaries
RUN curl -sSLfo /usr/local/bin/restic.bz2 \
    "https://github.com/restic/restic/releases/download/v${RESTIC_VERSION}/restic_${RESTIC_VERSION}_linux_amd64.bz2"  \
    && bzip2 -d /usr/local/bin/restic.bz2 \
    && chmod +x /usr/local/bin/restic

WORKDIR /bin
COPY backup.sh backup.sh
RUN chmod +x backup.sh

ENTRYPOINT ["./backup.sh"]