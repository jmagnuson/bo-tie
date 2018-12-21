set -ex

UNIT_TESTS_FILE=$( ls /workspace/bo-tie/target/$TARGET/debug/ | grep '^bo_tie.*[^d]$' )
LIBRARY_FILE=$()

UNIT_TESTS=/workspace/bo-tie/target/$TARGET/debug/$UNIT_TESTS_FILE

rsync -rc /workspace/bo-tie/ci/android/BoTieTester /workspace

cp $UNIT_TESTS /workspace/BoTieTester/app/src/main/assets/bo_tie_tests

cd /workspace/BoTieTester

./gradlew installDebug
