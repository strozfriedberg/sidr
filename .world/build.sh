#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' ]]; then
  exit
fi

if [[ "$Target" == 'linux' ]]; then
  $MAKE -j$PROCS build check OS=$Target ARCH=$Architecture LINKAGE=$Linkage INSTALL=$INSTALL DEPS=$DEPS
fi
