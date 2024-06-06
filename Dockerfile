# Usa la imagen base de Ubuntu 22.04
FROM ubuntu:22.04

RUN apt-get update && apt-get upgrade -y && \
    apt-get install -y curl git build-essential libicu-dev flex bison libreadline-dev zlib1g-dev

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

ENV PATH=/root/.cargo/bin:$PATH

RUN git clone -b dockerfile --single-branch https://github.com/isoprophlex/distributed-postgres.git /app

WORKDIR /app

RUN chmod +x ./init-and-start-sv.sh
