#!/bin/bash -ex

. .world/build_config.sh

BASEDIR=$(pwd)

if [[ "$Linkage" == 'static' ]]; then
  exit
fi

if [ "$Target" = 'linux' ]; then

  cp $BASEDIR/target/release/sidr${EXE_EXT} $INSTALL/bin

elif [ "$Target" = 'windows' ]; then

  cp $BASEDIR/target/x86_64-pc-windows-gnu/release/sidr.exe $INSTALL/bin

fi
