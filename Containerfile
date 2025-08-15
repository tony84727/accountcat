FROM node:24-alpine3.21 AS build_frontend

RUN apk --no-cache add make protoc sed
RUN npm install -g protoc-gen-js
WORKDIR /project
ADD ui .
RUN npm install
RUN make

FROM rust:1.89-alpine3.21 AS compile_server
RUN apk --no-cache add build-base openssl-dev openssl-libs-static protoc protobuf-dev
WORKDIR /project
ENV SQLX_OFFLINE=true
ADD . .
RUN --mount=type=cache,target=/root/.cargo --mount=type=cache,target=/project/target cargo build --release && mkdir -p /project/bin && cp /project/target/release/accountcat /project/bin/accountcat

FROM alpine:3.21
WORKDIR /opt/accountcat
COPY --from=compile_server /project/bin/accountcat .
COPY --from=build_frontend /project/dist ./ui/dist
CMD ["/opt/accountcat/accountcat", "server"]
