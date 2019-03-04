set -ex

LIBRARY_FILE=$( ls -tr /workspace/bo-tie/target/$TARGET/debug/deps | grep 'bo_tie' | tail -1 )

rsync -rc /workspace/bo-tie/ci/docker/android/instrument-tests/BoTieTester /workspace

mkdir -p /workspace/BoTieTester/app/?/
rsync -rc /.android /workspace/BoTieTester/app/?

# Move library file for linking android native instrument tests
if [ -d /workspace/BoTieTester/app/src/main/jniLibs ]
then
  rm -r /workspace/BoTieTester/app/src/main/jniLibs 2> /dev/null
fi

mkdir -p /workspace/BoTieTester/app/src/main/jniLibs/$ABI
mkdir -p /workspace/BoTieTester/app/src/androidTest//jniLibs/$ABI

cp /workspace/bo-tie/target/$TARGET/debug/deps/$LIBRARY_FILE /workspace/BoTieTester/app/src/main/jniLibs/$ABI/libbo-tie.so
cp /workspace/bo-tie/target/$TARGET/debug/deps/$LIBRARY_FILE /workspace/BoTieTester/app/src/androidTest/jniLibs/$ABI/libbo-tie2.so

# Move the java instrument tests to the folder
cp /workspace/bo-tie/temp/InstrumentTests.java /workspace/BoTieTester/app/src/androidTest/java/botietester

adb kill-server
adb start-server

cd /workspace/BoTieTester

./gradlew -q connectedAndroidTest
# ./gradlew clean
