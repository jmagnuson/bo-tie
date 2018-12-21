FROM ubuntu:18.04

ENV TARGET i686-linux-android
ENV ANDROID_HOME /android/sdk
ENV ANDROID_NDK /android/sdk/ndk-bundle

RUN apt-get update && apt-get install -y --no-install-recommends \
  curl \
  unzip \
  openjdk-8-jdk \
  python \
  rsync \
  gawk \
  gcc \
  clang \
  libc6-dev \
  bluetooth \
  libbluetooth3

ENV PATH $PATH:/android-toolchain/bin:/opt/kotlinc/bin

COPY ./docker /ci/docker

RUN mkdir -p /workspace

RUN chmod -R a+wrx /root

RUN chmod -R a+wrx /workspace

RUN sh ci/docker/targets/android-target-setup.sh i686-linux-android /android x86 26
