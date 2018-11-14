set -ex

SDK_SHA_CKSM="92ffee5a1d98d856634e8b71132e8a95d96c83a63fde1099be3d86df3106def9"
SDK_VERSION="4333796"

ANDROID_PTH=$( if [ $1 ]; then echo $1; else echo $( cd $(dirname $0) ; pwd -P )/android; fi )

TARGET=$( if [ $2 ]; then echo $2; else echo default; fi; )

# ARCH can be arm, arm64, x86, or x86_64, see the --arch option for
# make_standalone_toolchain.py
ARCH=$( if [ $3 ]; then echo $3; else echo arm64; fi; )

API=$( if [ $4 ]; then echo $4; else echo 26; fi; )

TARGET_PTH=$ANDROID_PTH/$TARGET

SDK_PTH=$ANDROID_PTH/sdk

qemu

# # This only checks that the zip file exists & the checksum matches
# if [ ! -f $SDK_PTH/sdk.zip ] || \
#    [ $( sha256sum $SDK_PTH/sdk.zip | gawk '/[:alnum:]+/ { printf "%s",$1 }' ) != $SDK_SHA_CKSM  ]
# then
#
#   if [ -d $SDK_PTH ]; then
#     rm -r $SDK_PTH
#   fi

  mkdir -p $SDK_PTH

  while [ $( sha256sum $SDK_PTH/sdk.zip | gawk '/[:alnum:]+/ { printf "%s",$1 }' ) != $SDK_SHA_CKSM ]
  do
    curl https://dl.google.com/android/repository/sdk-tools-linux-4333796.zip > $SDK_PTH/sdk.zip
  done

  unzip $SDK_PTH/sdk.zip -d $SDK_PTH

  yes | $SDK_PTH/tools/bin/sdkmanager --licenses > /dev/null

  if ! java -version > /dev/null 2>&1 ; then
    sudo apt install openjdk-8-jdk-headless
  fi

  yes | $SDK_PTH/tools/bin/sdkmanager ndk-bundle > /dev/null
  yes | $SDK_PTH/tools/bin/sdkmanager platform-tools > /dev/null

# fi

$SDK_PTH/ndk-bundle/build/tools/make_standalone_toolchain.py \
  --arch $ARCH \
  --stl=libc++ \
  --api $API \
  --install-dir $TARGET
