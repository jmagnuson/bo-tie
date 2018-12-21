set -ex


cd /workspace/bo-tie

if [ $BUILD_FOR_RELEASE = true ]
then
  RELEASE_FLAG='--release'
else
  RELEASE_FLAG=''
fi

if [ $RUN_RUST_TESTS = true ]
then
  cargo test ---target $TARGET $RELEASE_FLAG
elif [ $BUILD_RUST_TESTS = true ]
then
  cargo test --target $TARGET $RELEASE_FLAG --no-run
fi

cargo build --target $TARGET --release --lib
