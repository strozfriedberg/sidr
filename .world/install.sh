#!/bin/bash -ex

. .world/build_config.sh

BASEDIR=$(pwd)

if [[ "$Linkage" == 'static' ]]; then
  exit
fi

cp $BASEDIR/target/release/sidr${EXE_EXT} $INSTALL/bin
