FROM ubuntu:18.04

ENV TARGET aarch64-linux-android
ENV ANDROID_ARCH arm64
ENV ANDROID_SDK_VERSION 4333796

RUN apt-get update && apt-get install -y --no-install-recommends \
  curl \
  ca-certificates \
  unzip \
  openjdk-8-jre \
  python \
  gcc \
  libc6-dev

COPY android_ndk.sh /opt

RUN /opt/android_ndk.sh /opt aarch64-linux-android

ENV ANDROID_SDK /opt/android-sdk
