FROM ubuntu:18.04

ENV TARGET x86_64-linux-android
ENV ANDROID_ARCH x86_64

RUN apt-get update && apt-get install -y --no-install-recommends \
  curl \
  ca-certificates \
  unzip \
  openjdk-8-jre \
  python \
  gcc \
  libc6-dev \
  gawk

WORKDIR /ci

EXPOSE 5037

COPY . /ci

ENV PATH $PATH:/android-toolchain/bin

ENV CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=x86_64-linux-android-gcc \
    CARGO_TARGET_X86_64_LINUX_ANDROID_RUNNER="true"

RUN sh ./android-emulator-setup.sh /android x86_64-linux-android x86_64 26
