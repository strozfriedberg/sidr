#!/bin/bash -ex

. .world/build_config.sh

BASEDIR=$(pwd)

if [[ "$Linkage" == 'static' ]]; then
  exit
fi

if [[ "$Target" == 'linux' || "$Target" == 'windows_package' ]]; then
  mkdir -p $INSTALL/bin
  cp $BASEDIR/target/release/sidr${EXE_EXT} $INSTALL/bin
fi
