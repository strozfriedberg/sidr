#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' ]]; then
  exit
fi

if [[ "$Target" == 'linux' ]]; then
  $MAKE -j$PROCS setup OS=$Target ARCH=$Architecture LINKAGE=$Linkage INSTALL=$INSTALL DEPS=$DEPS
elif [[ "$Target" == 'macos' ]]; then
  $MAKE -j$PROCS macos_setup OS=$Target ARCH=$Architecture LINKAGE=$Linkage INSTALL=$INSTALL DEPS=$DEPS
fi
