set -ex

if [ $RUN_RUST_TESTS = true ]
then
(
  cd /workspace/bo-tie

  cargo test ---target $TARGET
)
fi

case $TARGET in
  arm-linux-androideabi)
  JNI_LIB_FOLDER='armeabi'
  ;;
  armv7-linux-androideabi)
  JNI_LIB_FOLDER='armeabi-v7a'
  ;;
  aarch64-linux-android)
  JNI_LIB_FOLDER='arm64-v8a'
  ;;
  i686-linux-android)
  JNI_LIB_FOLDER='x86'
  ;;
  x86_64-linux-android)
  JNI_LIB_FOLDER='x86_64'
  ;;
esac

mkdir -p /ci/targets

cd /workspace/bo-tie-tests

cargo build --release --target $TARGET

cp -r target/$TARGET /ci/targets
