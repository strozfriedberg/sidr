#!/bin/bash -ex

. .world/build_config.sh

BASEDIR=$(pwd)

if [[ "$Linkage" == 'static' ]]; then
  exit
fi

ls /home/builder/make_world/world/sidr/target/release/
cp $BASEDIR/target/release/sidr${EXE_EXT} $INSTALL/bin
