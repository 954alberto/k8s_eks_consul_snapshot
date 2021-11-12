FROM debian:stable

RUN apt-get update && apt-get -y install ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

RUN groupadd --gid 1000 -r consnap && useradd --uid 1000 -r -g consnap consnap

COPY consnap /home/consnap/consnap

RUN chown -R consnap:consnap /home/consnap

WORKDIR /home/consnap/

USER 1000