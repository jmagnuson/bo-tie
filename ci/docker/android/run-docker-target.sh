#!/bin/sh
set -ex

TARGET="android-$1"
TARGET_FOLDER=$(dirname $0)/docker/android/$1

# echo the linker for the specific target
#
# Used for special cases (armv7-linux-androideabi -> arm-linux-androideabi)
# usually this just prints the same target as the argument
#
# Takes the target as the argument. Argument is required
echo_linker() {
  case $1 in
    armv7-linux-androideabi)
    echo arm-linux-androideabi
    ;;
    *)
    echo $1
    ;;
  esac
}

# For building/running the required docker container to generate the library
# required for linking and testing the unit tests
run_depend_target() {

  if [ $BUILD_FLAG = true ] || [[ -z $(docker image ls -q bo-tie:$1) ]]
  then
    local DOCKER_FILE=$(dirname $0)/docker/targets/$1/Dockerfile
    docker build -t "bo-tie:$1" -f $DOCKER_FILE $(dirname $0)
  fi

  docker run \
    --rm \
    --user `id -u`:`id -g` \
    --init \
    --privileged \
    --volume $HOME/.cargo:/cargo \
    --env CARGO_HOME=/cargo \
    --volume `rustc --print sysroot`:/rust:ro \
    --env TARGET=$1 \
    --env CARGO_TARGET_$(echo $1 | tr '[:lower:]-' '[:upper:]_')_LINKER=$(echo_linker $1)-gcc \
    --env CARGO_TARGET_$(echo $1 | tr '[:lower:]-' '[:upper:]_')_RUNNER="true" \
    --env JNI_INCLUDE=/android-toolchain/sysroot/usr/include \
    --env GEN_JAVA_FILE_PATH=/workspace/bo-tie/temp \
    --env LIBRARY_FILE_NAME=bo-tie \
    --env JAVA_PACKAGE='botietester' \
    --env ANDROID_JAR_CLASSPATH='/android/sdk/platforms/android-28/android.jar' \
    --volume $BO_TIE_PATH:/workspace/bo-tie \
    --workdir "/workspace/bo-tie" \
    bo-tie:$1 \
    bash -c \
    'PATH=$PATH:/rust/bin exec cargo rustc --features android_test --target $TARGET -- --crate-type dylib'
}

if [ $BUILD_FLAG = true ] || [[ -z $(docker image ls -q bo-tie:$TARGET) ]]
then
  docker build -t "bo-tie:$TARGET" -f $TARGET_FOLDER/Dockerfile $(dirname $0)
fi

if [ $RUN_FLAG = true ]
then
  TARGET_FOLDER_PATH=/workspace/bo-tie/target

  adb start-server

  ABI=$(adb shell getprop ro.product.cpu.abi)

  case $ABI in
    armeabi)
    DEPENDENCY_TARGET=arm-linux-androideabi
    ;;
    armeabi-v7a)
    DEPENDENCY_TARGET=armv7-linux-androideabi
    ;;
    arm64-v8a)
    DEPENDENCY_TARGET=aarch64-linux-android
    ;;
    x86)
    DEPENDENCY_TARGET=i686-linux-android
    ;;
    x86_64)
    DEPENDENCY_TARGET=x86_64-linux-android
    ;;
    *)
    exit
    ;;
  esac

  run_depend_target $DEPENDENCY_TARGET

  CONTAINER_ID=$(docker create \
    --user `id -u`:`id -g` \
    --group-add plugdev \
    --init \
    --privileged \
    --network=host \
    --volume $HOME/.cargo:/cargo \
    --env CARGO_HOME=/cargo \
    --env ABI=$ABI \
    --env TARGET=$DEPENDENCY_TARGET \
    --volume `rustc --print sysroot`:/rust:ro \
    --volume $BO_TIE_PATH:/workspace/bo-tie:ro \
    --volume $HOME/.android:/.android \
    bo-tie:$TARGET \
    bash -c \
    'PATH=$PATH:/rust/bin exec sh /workspace/bo-tie/ci/docker/android/instrument-tests/run.sh')

  docker start -a $CONTAINER_ID || true # Continue even if this fails

  docker commit $CONTAINER_ID bo-tie:$TARGET

  docker rm $CONTAINER_ID
fi
