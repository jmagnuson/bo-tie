set -ex

if [[ -z $(docker image ls -q classbytes) ]]; then
  docker build -t classbytes $(dirname $0)
fi;

mkdir -p target
docker run \
  --user `id -u`:`id -g` \
  --rm \
  --init \
  --volume $HOME/.cargo:/cargo \
  --env CARGO_HOME=/cargo \
  --env ANDROID_SDK_PATH='/android' \
  --volume `rustc --print sysroot`:/rust:ro \
  --volume $(cd `dirname $0`/../../../../ ; pwd -P ):/workspace:ro \
  --volume $(cd `dirname $0`/.. ; pwd -P ):/workspace/subcrates/android/classbytes \
  --volume $(cd `dirname $0` ; pwd -P)/../../../../src/android/classbytes.rs:/workspace/src/android/classbytes.rs \
  --workdir /workspace/subcrates/android/classbytes \
  classbytes \
  bash -c 'PATH=$PATH:/rust/bin exec cargo run'
