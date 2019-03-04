#!/bin/sh
# This file requires one argument, one of the target-triples (listed as
# folder names in this directory) to build

set -ex

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

# Run target-triple image
run_trip_target_image() {
  docker run \
    --rm \
    --user `id -u`:`id -g` \
    --net=host \
    --init \
    --privileged \
    --volume $HOME/.cargo:/cargo \
    --env CARGO_HOME=/cargo \
    --volume `rustc --print sysroot`:/rust:ro \
    --env TARGET=$1 \
    --env CARGO_TARGET_$(echo $1 | tr '[:lower:]-' '[:upper:]_')_LINKER=$(echo_linker $1)-gcc \
    --env CARGO_TARGET_$(echo $1 | tr '[:lower:]-' '[:upper:]_')_RUNNER="true" \
    --env BUILD_FOR_RELEASE=$BUILD_FOR_RELEASE \
    --env RUN_RUST_TESTS=$RUN_RUST_TESTS \
    --env BUILD_RUST_TESTS=$BUILD_RUST_TESTS \
    --env BUILD_DOC=$BUILD_DOC \
    --volume $BO_TIE_PATH:/workspace/bo-tie:ro \
    --volume $BO_TIE_PATH/Cargo.lock:/workspace/bo-tie/Cargo.lock \
    --volume $BO_TIE_PATH/target:/workspace/bo-tie/target \
    bo-tie:$1 \
    bash -c \
    'PATH=$PATH:/rust/bin exec sh /workspace/bo-tie/ci/docker/targets/run.sh'
}

# Runs docker actions for the target triples
#
# Takes one argument, the target triple to build
#
# Creates the docker container
#
# If the image isn't build or the build image option is specified by the user
# of the script then the image is build before a container is built from the
# the target specific image.
if [ $BUILD_FLAG = true ] || [[ -z $(docker image ls -q bo-tie:$1) ]]
then
  DOCKER_FILE=$(dirname $0)/docker/targets/$1/Dockerfile
  docker build -t "bo-tie:$1" -f $DOCKER_FILE $(dirname $0)
fi

if [ $RUN_FLAG = true ]
then

  mkdir -p $BO_TIE_PATH/target

  run_trip_target_image $1

  if [ $BUILD_DOC = true ]
  then
    xdg-open $BO_TIE_PATH/target/$1/doc/bo_tie/index.html &
  fi
fi
