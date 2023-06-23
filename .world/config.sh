#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' ]]; then
  exit
fi

if [[ "$Target" == 'linux' ]]; then
  $MAKE -j$PROCS config OS=$Target ARCH=$Architecture LINKAGE=$Linkage INSTALL=$INSTALL DEPS=$DEPS
elif [[ "$Target" == 'macos' ]]; then
  $MAKE -j$PROCS macos_config OS=$Target ARCH=$Architecture LINKAGE=$Linkage INSTALL=$INSTALL DEPS=$DEPS
fi
