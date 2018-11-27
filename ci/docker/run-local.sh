# This run script is for running both the rust unit tests and 'local' unit tests
# of the android TestProject app

set -ex

if [ $RUN_RUST_TESTS = true ]
then
  (
    cd bo-tie

    # Run rust only tests
    cargo test --release --target $TARGET
  )
fi

# Need to build the test project bo-tie-tests in order to perform either
if [ $RUN_ANDROID_LOCAL_TESTS = true ] || [ $RUN_ANDROID_INSTRUMENT_TESTS = true ]
then
(
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

  mkdir -p TestProject/app/src/main/jniLibs/$JNI_LIB_FOLDER

  cd bo-tie-tests

  cargo build --release --lib

  cp target/release/libbo_tie_tests.so ../TestProject/app/src/main/jniLibs/$JNI_LIB_FOLDER/libbo_tie_tests.so
)
fi

if [ $RUN_ANDROID_LOCAL_TESTS = true ]
then
(
  set -x
  cd TestProject

  ./gradlew test
)
fi

if [ $RUN_ANDROID_INSTRUMENT_TESTS = true ]
then
(
  echo "instrumental testing isn't enabled yet"

  # # kill adb server if its running
  # /ci/android/platform-tools/adb kill-server
  #
  # # Start the emulator
  # (
  #   /ci/android/tools/emulator -avd bo-tie \
  #     -noaudio \
  #     -no-window \
  #     -gpu off \
  #     -qemu \
  #     -no-cache \
  #     -qemu -bt hci,host \
  # )
  #
  # # Start Server
  # /ci/android/platform-tools/adb start-server
  #
  # # Build and install the TestProject (in debug mode)
  # /ci/android/TestProject/gradlew installDebug
  #
  # # Run the tests
  # /ci/android/platform-tools/adb -d
)
fi
