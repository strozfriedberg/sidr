#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' ]]; then
  exit
fi

BASEDIR=$(pwd)
WSA_TEST_DB_PATH=$BASEDIR/tests/testdata WSA_TEST_CONFIGURATION_PATH=$BASEDIR/src/bin/test_reports_cfg.yaml cargo test

if [ "$Target" = 'linux' ]; then

  cargo build -r

elif [ "$Target" = 'windows' ]; then

  rustup target add x86_64-pc-windows-gnu
  cargo build -r --target x86_64-pc-windows-gnu

fi

ls /home/builder/make_world/world/sidr/target/release/
