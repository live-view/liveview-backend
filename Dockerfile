FROM rust:1.83.0 AS builder
WORKDIR /app
RUN cargo init
COPY ./ ./
RUN cargo build --release

FROM ubuntu:24.10
ADD https://github.com/ufoscout/docker-compose-wait/releases/download/2.12.1/wait /wait
RUN chmod +x /wait
WORKDIR /app
COPY ./ ./
COPY --from=builder /app/target/release/liveview-backend /app
CMD [ "sh", "./entrypoint.sh" ]