# Default operation
BUILD_FLAG=false
RUN_FLAG=true

BUILD_FOR_RELEASE=false
RUN_RUST_TESTS=false
BUILD_RUST_TESTS=false
BUILD_ANDROID_TEST_WRAPPER=false
RUN_ANDROID_LOCAL_TESTS=true
RUN_ANDROID_INSTRUMENT_TESTS=false
BUILD_DOC=false

# So that run-docker.sh can be run from the bo-tie directory or ci directory
if [ $(dirname $0) = '.' ]
then
  BO_TIE_PATH=`pwd`/..
else
  BO_TIE_PATH=`pwd`
fi

# Search for dockerfiles from the given basepath
#
# Returns a list of Dockerfile paths relative to the provided basepath (but the
# dot isn't printed before the first path slash).
#
# Requires 1 argument, the basepath from where to start searching.
# A second argument can be passed if it is desired to prepend each path in the
# returned list with a given path.
dockerfile_search() {
  local DOCKERS=()

  # P in the following 2 variable names is short for PATH
  local P=$(if [ $1 ]; then echo $1; else echo '.'; fi)
  local OUT_P=$(if [ $2 ]; then echo $2; else echo ''; fi)

  for F in $(ls $P); do
    if [ -d $P/$F ]; then
        DOCKERS=( ${DOCKERS[*]} $(dockerfile_search $P/$F $OUT_P/$F) )
    else
      if [ $F = Dockerfile ]; then
        DOCKERS=( ${DOCKERS[*]} $OUT_P )
      fi
    fi
  done

  echo ${DOCKERS[*]}
}

# Prints out the available targets, for use with the --print-targets option
#
# No args
print_targets() {
  local TARGET_PATHS=(\
    $(dockerfile_search $BO_TIE_PATH/ci/docker/android android) \
    $(dockerfile_search $BO_TIE_PATH/ci/docker/targets) \
  )
  local TARGET_PARTS=
  local TARGET=
  local ALL_TARGETS=

  for TP in ${TARGET_PATHS[*]}; do
    # SPLIT by /
    IFS="\/" read -ra TARGET_PARTS <<< $TP
    unset IFS

    for I in ${TARGET_PARTS[*]}; do
      if [ "$TARGET" ]; then
        TARGET="$TARGET-$I"
      else
        TARGET="$I"
      fi
    done

    ALL_TARGETS=( ${ALL_TARGETS[@]} $TARGET )

    TARGET=''
  done

  printf "%s\n" $(echo ${ALL_TARGETS[@]} | tr " " "\n" | sort)
}

HELP_MESSAGE="""Usage: run-docker.sh [OPTION]

This script runs the local unit tests for all android targets unless -t or
--target are specified. The docker images are only built if the image doesn't
exist.

Options:
-b, --build               Build the docker image(s).
    --no-run              Do not run the container(s), usefull when combined
                          with -b or --build. All other options that require a
                          docker container will have no effect.
-r, --release             Add the --release flag to rust tests and builds.
    --rust-tests          Run rust tests
    --build-rust-tests    Build rust tests but do not run rust tests. Superseded
                          by the flag '--rust-tests'
    --no-android-local-tests
                          Do not run the android local tests. This is only
                          applicable for the android-local-tests target.
-i, --no-android-instrument-tests
                          Run the tests through the android emulator. This is
                          only applicable for the android-instrument-tests
                          targert (which is still being developed).
-t, --target <TARGET>     Run for only the specified target, this can be
                          used multiple times for multiple targets.
    --print-targets       Print all docker targets
    --doc                 Build and open documentation. Only applicable to build
                          targets and not test targets. Does nothing if not
                          applicable.
-h, --help                This message.
"""

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
    -r|--release)
    BUILD_FOR_RELEASE=true
    shift
    ;;
    --rust-tests)
    RUN_RUST_TESTS=true
    shift
    ;;
    --build-rust-tests)
    BUILD_RUST_TESTS=true
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
    --doc)
    BUILD_DOC=true
    shift
    ;;
    -h|--help)
    echo "$HELP_MESSAGE"
    exit
    ;;
    --print-targets)
    print_targets
    exit
    ;;
    *)
    echo "run-docker.sh: unrecognized option: $1"
    echo "Try 'run-docker.sh --help' for more information "
    exit
    ;;
  esac
done

find_docker_runners() {
  for F in $(ls $1)
  do
    if [ -f $1/$F/Dockerfile ]
    then
      source $1/run-docker-target.sh $F
    elif [ -d $1/$F ]
    then
      find_docker_runners "$1/$F"
    fi
  done
}

set -ex

if [ ! -z ${TARGETS:+x} ]; then
  for T in $TARGETS; do
    ANDROID_PREFIX='android-'
    case $T in
      $ANDROID_PREFIX*)
      source $(dirname $0)/docker/android/run-docker-target.sh ${T:${#ANDROID_PREFIX}}
      ;;
      *)
      source $(dirname $0)/docker/targets/run-docker-target.sh $T
      ;;
    esac
  done
else
  # All targets in ci/docker/targets
  find_docker_runners $(dirname $0)/docker
fi
