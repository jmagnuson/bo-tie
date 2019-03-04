FROM ubuntu:18.04

RUN apt-get update && apt-get install -y --no-install-recommends \
  curl \
  unzip \
  openjdk-8-jdk \
  python \
  gcc \
  clang \
  gawk

ENV SDK_SHA_CKSM "92ffee5a1d98d856634e8b71132e8a95d96c83a63fde1099be3d86df3106def9"
ENV SDK_VERSION "4333796"
ENV SDK_PTH /android

RUN mkdir -p $SDK_PTH; \
  curl https://dl.google.com/android/repository/sdk-tools-linux-$SDK_VERSION.zip > $SDK_PTH/sdk.zip; \
  while [ $( sha256sum $SDK_PTH/sdk.zip | gawk '/[:alnum:]+/ { printf "%s",$1 }' ) != $SDK_SHA_CKSM ]; \
  do \
    curl https://dl.google.com/android/repository/sdk-tools-linux-$SDK_VERSION.zip > $SDK_PTH/sdk.zip; \
  done; \
  unzip -q $SDK_PTH/sdk.zip -d $SDK_PTH; \
  rm $SDK_PTH/sdk.zip; \
  yes | $SDK_PTH/tools/bin/sdkmanager --licenses > /dev/null; \
  yes | $SDK_PTH/tools/bin/sdkmanager 'platforms;android-28' 'build-tools;28.0.3' > /dev/null
