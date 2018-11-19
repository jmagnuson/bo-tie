# Default operation
BUILD_FLAG=false
RUN_FLAG=true

for i in "$@"; do
  case $i in
    -b|--build)
    BUILD_FLAG=true
    shift
    ;;
    --no-run)
    BUILD_FLAG=true
    RUN_FLAG=false
    shift
    ;;
    -h|--help)
    echo """Usage: run-docker.sh [OPTION]

    Arguments:
    -b, --build               Build and run the docker image.
        --no-run              Build the image and do not run it.
    -h, --help                This message.
    """
    exit
    ;;
    *)
    echo "Unknown flag: $1"
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
  if [ $BUILD_FLAG = true ] || [ $(docker image inspect bo-tie > /dev/null 2>&1) ]
  then
    docker build -t bo-tie -f $(dirname $0)/docker/$1/Dockerfile $(dirname $0)
  fi

  mkdir -p target

  if [ $RUN_FLAG = true ]
  then
    docker run \
      --user `id -u`:`id -g` \
      --rm \
      --init \
      --volume $HOME/.cargo:/cargo \
      --env CARGO_HOME=/cargo \
      --volume `rustc --print sysroot`:/rust:ro \
      --env TARGET=$1 \
      --volume $BO_TIE_PATH:/workspace:ro \
      --volume $BO_TIE_PATH/target:/workspace/target \
      --workdir /workspace \
      --privileged \
      bo-tie \
      bash \
      -c 'PATH=$PATH:/rust/bin exec sh ci/run.sh $TARGET'
  fi
}

run x86_64-linux-android
