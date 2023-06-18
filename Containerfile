FROM docker.io/library/rust:bullseye as builder

RUN apt-get update && \
    apt-get install --yes \
      cmake

COPY . .
RUN cargo install \
      --path . \
      --root /usr/local

FROM docker.io/library/debian:bullseye-slim

RUN apt-get update && \
    apt-get install --yes \
      ca-certificates \
      tini

COPY --from=builder \
  /usr/local/bin/spaceapi-matrix-bot \
  /usr/local/bin/spaceapi-matrix-bot

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/spaceapi-matrix-bot"]
