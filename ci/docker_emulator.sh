#!/bin/bash
set -ex

# LIST of available devices are on the 'butomo1989/docker-android' github)
DEVICE=$( if [ $1 ]; then echo $1; else echo "Samsung Galaxy S6"; fi; )


CON_DEV_NAME=$(sed -e 's/ /-/g' <<< $DEVICE)

docker run --privileged \
  -d \
  -p 6080:6080 \
  -p 5554:5554 \
  -p 5555:5555 \
  -e DEVICE="$DEVICE" \
  --name android-container-$CON_DEV_NAME \
  --user `id -u`:`id -g` \
  --rm \
  butomo1989/docker-android-x86-8.1 \
  > /dev/null 2>&1
