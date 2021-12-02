FROM rust:1.56.1-bullseye

 MAINTAINER Kraktus

 # Based on https://blog.logrocket.com/packaging-a-rust-web-service-using-docker/
 RUN mkdir rs_retroboard
 WORKDIR ./rs_retroboard


 RUN apt-get update && apt-get upgrade -y

# Install flamegraph

RUN apt install -y linux-perf
RUN cargo install flamegraph

 # Build dependencies
 COPY ./Cargo.toml ./Cargo.toml
 RUN mkdir src && echo "fn main() {}" > src/main.rs 
 RUN cargo fetch
 RUN cargo build
 RUN rm -rf src

 # Based on https://github.com/rust-lang/docs.rs/blob/263c00d3dd01e68c38f3ec4a5e27979825e301a8/dockerfiles/Dockerfile#L41
 # Dependencies are now cached, copy the actual source code and do another full
 # build. The touch on all the .rs files is needed, otherwise cargo assumes the
 # source code didn't change thanks to mtime weirdness.
 COPY src src/
 COPY README.md README.md
 RUN find src -name "*.rs" -exec touch {} \;
 RUN cargo build


 # docker build --force-rm -t rs-retroboard-image .
 # docker run -it --init --rm --name rs-retroboard-cont rs-retroboard-image

# docker run --cap-add SYS_ADMIN -it --name rs-retroboard-cont rs-retroboard-image
