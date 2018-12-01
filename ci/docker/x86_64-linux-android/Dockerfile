FROM ubuntu:18.04

ENV TARGET x86_64-linux-android
ENV ANDROID_HOME /android/sdk
ENV ANDROID_NDK /android/sdk/ndk-bundle

RUN apt-get update && apt-get install -y --no-install-recommends \
  curl \
  unzip \
  openjdk-8-jre \
  python \
  rsync \
  gawk \
  bluetooth \
  libbluetooth-dev

ENV PATH $PATH:/android-toolchain/bin:/opt/kotlinc/bin

ENV CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=x86_64-linux-android-gcc \
    CARGO_TARGET_X86_64_LINUX_ANDROID_RUNNER="true"

COPY ./docker /ci/docker

RUN mkdir -p /workspace

RUN chmod -R a+wrx /workspace

RUN sh ci/docker/android-setup.sh x86_64-linux-android /android x86_64 26
