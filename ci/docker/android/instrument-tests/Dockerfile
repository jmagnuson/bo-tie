FROM ubuntu:18.04

RUN apt-get update && apt-get install -y --no-install-recommends \
  curl \
  ca-certificates \
  openjdk-8-jdk \
  python \
  unzip \
  gcc \
  gawk \
  clang \
  libc6-dev \
  rsync \
  adb

COPY ./docker/android/instrument-tests /ci

EXPOSE 5037

RUN sh /ci/setup.sh /android
