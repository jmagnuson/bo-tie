set -ex

if [ $RUN_RUST_TESTS = true ]
then
  (
    cd bo-tie

    cargo test --release --target $TARGET
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

mkdir -p /targets/$TARGET

cd /workspace/bo-tie-tests

cargo build --release

cp target/release/libbo_tie_tests.so /targets/$TARGET
