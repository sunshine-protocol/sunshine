# src: https://github.com/paritytech/substrate/blob/master/.maintain/Dockerfile

FROM phusion/baseimage:0.11 as builder
LABEL maintainer="amar@sunshinelabs.io"
LABEL description="This is the build stage for Sunshine Chain Node. Here we create the binary."

ENV DEBIAN_FRONTEND=noninteractive

ARG PROFILE=release
WORKDIR /sunshine

COPY . /sunshine

RUN apt-get update && \
	apt-get dist-upgrade -y -o Dpkg::Options::="--force-confold" && \
	apt-get install -y cmake pkg-config libssl-dev git clang

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
	export PATH="$PATH:$HOME/.cargo/bin" && \
	rustup toolchain install nightly && \
	rustup target add wasm32-unknown-unknown --toolchain nightly && \
	rustup default stable && \
	cargo build "--$PROFILE"

# ===== SECOND STAGE ======

FROM phusion/baseimage:0.11
LABEL maintainer="amar@sunshinelabs.io"
LABEL description="This is the 2nd stage: a small image where we copy the Sunshine Node binary."
ARG PROFILE=release

RUN mv /usr/share/ca* /tmp && \
	rm -rf /usr/share/*  && \
	mv /tmp/ca-certificates /usr/share/ && \
	useradd -m -u 1000 -U -s /bin/sh -d /sunshine sunshine && \
	mkdir -p /sunshine/.local/share/sunshine && \
	chown -R sunshine:sunshine /sunshine/.local && \
	ln -s /sunshine/.local/share/sunshine /data

COPY --from=builder /sunshine/target/$PROFILE/sunshine-node /usr/local/bin

# checks
RUN ldd /usr/local/bin/sunshine-node && \
	/usr/local/bin/sunshine-node --version

# Shrinking
RUN rm -rf /usr/lib/python* && \
	rm -rf /usr/bin /usr/sbin /usr/share/man

USER sunshinedev
EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/sunshine-node"]