set -ex

if [ $RUN_LOCAL_TESTS = true ]
then
  sh $(dirname $0)/run-local.sh
fi

if [ $RUN_INSTRUMENT_TESTS = true ]
then
  sh $(dirname $0)/run-instrument.sh
fi
