set -ex

SDK_SHA_CKSM="92ffee5a1d98d856634e8b71132e8a95d96c83a63fde1099be3d86df3106def9"
SDK_VERSION="4333796"
KOTLIN_VERSION="1.3.10"

# Inputs to android-emulator-setup.sh
TARGET=$( if [ $1 ]; then echo $1; else echo default; fi; )
ANDROID_PTH=$( if [ $2 ]; then echo $2; else echo $( cd $(dirname $0) ; pwd -P )/android; fi )
# ARCH can be arm, arm64, x86, or x86_64, see the --arch option for
# $SDK_PTH/ndk-bundle/build/tools/make_standalone_toolchain.py
ARCH=$( if [ $3 ]; then echo $3; else echo x86_64; fi; )
API=$( if [ $4 ]; then echo $4; else echo 26; fi; )

# Android system-image related
SI_TARGET=$( if [ $SI_TARGET ]; then echo $SI_TARGET; else echo android-26; fi )
SI_TAG=$( if [ $SI_TAG ]; then echo $SI_TAG; else echo google_apis; fi )
SI_ABI=$( if [ $SI_ABI ]; then echo $SI_ABI; else echo x86_64; fi )

TARGET_PTH=$ANDROID_PTH/$TARGET

SDK_PTH=$ANDROID_PTH/sdk

# Get the android sdk
mkdir -p $SDK_PTH

curl https://dl.google.com/android/repository/sdk-tools-linux-$SDK_VERSION.zip > $SDK_PTH/sdk.zip

while [ $( sha256sum $SDK_PTH/sdk.zip | gawk '/[:alnum:]+/ { printf "%s",$1 }' ) != $SDK_SHA_CKSM ]
do
  curl https://dl.google.com/android/repository/sdk-tools-linux-$SDK_VERSION.zip > $SDK_PTH/sdk.zip
done

unzip -q $SDK_PTH/sdk.zip -d $SDK_PTH

rm $SDK_PTH/sdk.zip

# install android needed tools
yes | $SDK_PTH/tools/bin/sdkmanager --licenses > /dev/null
yes | $SDK_PTH/tools/bin/sdkmanager "system-images;$SI_TARGET;$SI_TAG;$SI_ABI" > /dev/null

# Default is no, which is what is wanted. Spamming no however doesn't work so
# just spamming newline instead with 'yes'.
yes '' | $SDK_PTH/tools/bin/avdmanager create avd \
      -k "system-images;$SI_TARGET;$SI_TAG;$SI_ABI" \
      --force \
      -n bo-tie \
      -b $SI_ABI \
      -g $SI_TAG \
      > /dev/null
