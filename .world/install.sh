#!/bin/bash -ex

. .world/build_config.sh

BASEDIR=$(pwd)

if [[ "$Linkage" == 'static' ]]; then
  exit
fi

cargo build -r

if [ "$Target" = 'linux' ]; then

  cp $BASEDIR/target/release/sidr $INSTALL/bin

elif [ "$Target" = 'windows' ]; then

  cp $BASEDIR/target/release/sidr.exe $INSTALL/bin

fi