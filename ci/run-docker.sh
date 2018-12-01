# Default operation
BUILD_FLAG=false
RUN_FLAG=true

RUN_RUST_TESTS=true
RUN_ANDROID_LOCAL_TESTS=true
RUN_ANDROID_INSTRUMENT_TESTS=false

while [[ $# -gt 0 ]]; do
  key=$1
  case $key in
    -b|--build)
    BUILD_FLAG=true
    shift
    ;;
    --no-run)
    BUILD_FLAG=true
    RUN_FLAG=false
    shift
    ;;
    --no-rust-tests)
    RUN_RUST_TESTS=false
    shift
    ;;
    --no-local-tests)
    RUN_ANDROID_LOCAL_TESTS=false
    shift
    ;;
    -i|--instrument)
    RUN_ANDROID_INSTRUMENT_TESTS=true
    shift
    ;;
    -t|--target)
    TARGETS=( "${TARGETS[@]}" "$2" )
    shift 2
    ;;
    -h|--help)
    echo """Usage: run-docker.sh [OPTION]

This script runs the local unit tests for all android targets. By default,
docker images are only built if the image doesn't exist, and the instrument
tests not run on an emulator. Options can be specified to change the default
behavior.

Options:
  -b, --build               Build and run the docker image.
      --no-run              Build the image and do not run it. All other
                            options that require the image to run will have
                            no effect.
      --no-rust-tests       Do not run rust tests
      --no-local-tests      Do not run the local tests
  -i, --instrument          Run the tests through the android emulator
  -t, --target <TRIPLE>     Run for only the specified target, this can be
                            used multiple times for multiple targets.

  -h, --help                This message.

Note:
Android targets do not run rust tests because the docker environment is not
Android.
"""
    exit
    ;;
    *)
    echo "run-docker.sh: unrecognized option: $1"
    echo "Try 'run-docker.sh --help' for more information "
    exit
    ;;
  esac
done

set -ex

# So that run-docker.sh can be run from the bo-tie directory or ci directory
if [ $(dirname $0) = '.' ]
then
  BO_TIE_PATH=`pwd`/..
else
  BO_TIE_PATH=`pwd`
fi

run() {
  TARGET=$1

  if [ $BUILD_FLAG = true ] || [[ -z $(docker image ls -q bo-tie:$TARGET) ]]
  then
    docker build -t "bo-tie:$TARGET" -f $(dirname $0)/docker/$TARGET/Dockerfile $(dirname $0)
  fi

  mkdir -p $BO_TIE_PATH/target

  if [ $RUN_FLAG = true ]
  then
    CONTAINER_ID=$(docker create \
      --user `id -u`:`id -g` \
      --net=host \
      --init \
      --volume $HOME/.cargo:/cargo \
      --env CARGO_HOME=/cargo \
      --volume `rustc --print sysroot`:/rust:ro \
      --env TARGET=$TARGET \
      --env CARGO_TARGET_$(echo $TARGET | tr '[:lower:]-' '[:upper:]_')_LINKER=${TARGET}-gcc \
      --env CARGO_TARGET_$(echo $TARGET | tr '[:lower:]-' '[:upper:]_')_RUNNER="true" \
      --env RUN_RUST_TESTS=$RUN_RUST_TESTS \
      --env RUN_ANDROID_LOCAL_TESTS=$RUN_ANDROID_LOCAL_TESTS \
      --env RUN_ANDROID_INSTRUMENT_TESTS=$RUN_ANDROID_INSTRUMENT_TESTS \
      --env JNI_INCLUDE=/android-toolchain/sysroot/usr/include \
      --env RUN_LOCAL_TESTS=true \
      --env RUN_INSTRUMENT_TESTS=false \
      --volume $BO_TIE_PATH:/workspace/bo-tie:ro \
      --volume $BO_TIE_PATH/target:/workspace/bo-tie/target \
      --volume $BO_TIE_PATH/ci/targets:/ci/targets \
      --privileged \
      bo-tie:$TARGET \
      bash -c \
      'rsync -rc --update /workspace/bo-tie/ci/android/{bo-tie-tests,TestProject} /workspace; \
       PATH=$PATH:/rust/bin exec sh /workspace/bo-tie/ci/docker/run.sh')

    # Need to commit to image regardless of
    docker start -a $CONTAINER_ID || true

    docker commit $CONTAINER_ID bo-tie:$TARGET

    docker rm $CONTAINER_ID
  fi
}

if [ ! -z ${TARGETS:+x} ]; then
  echo "Running bo-tie docker container for target ${TARGETS[@]}"
  for T in $TARGETS; do
    run $T
  done
else
  for F in $(ls $(dirname $0)/docker); do
    if [ -d $(dirname $0)/docker/$F ]; then
      run $F
    fi
  done
fi
