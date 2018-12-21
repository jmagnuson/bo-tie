set -ex

sh /workspace/bo-tie/ci/docker/targets/run.sh

if [ $BUILD_ANDROID_TEST_WRAPPER = true ]
then
  cd /workspace/bo-tie-tests

  cargo build --release --target $TARGET $RELEASE_FLAG
fi
