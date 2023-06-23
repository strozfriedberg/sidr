#!/bin/bash -ex

. .world/build_config.sh

BASEDIR=$(pwd)

if [[ "$Linkage" == 'static' ]]; then
  if [[ "$Target" == 'windows' && "$Architecture" == '64' ]]; then
    # copy statically linked 64-bit EXEs into bin for package_windows task
    mkdir -p $BASEDIR/asdf/bin
    cp -av $DEPS/bin/*.exe $BASEDIR/asdf/bin
  fi
  exit
fi

PYTHON=python3

for PROJECT in "$BASEDIR"/world_ci/e2e_scripts/*; do
  pushd "$PROJECT"
  $PYTHON make_e2e_scripts.py $Target
  chmod a+x *.sh
  popd
done

#
# Define variables for archive name
#

DATE=$(date +%Y%m%d)
pushd ..
GIT_WORLD_BRANCH=$(git branch --show-current)
GIT_WORLD_COMMIT=$(git rev-parse --short HEAD)
popd

if [ "$Target" = 'windows' ]; then
  PROJ_ARCHIVE_EXT=7z
  E2E_ARCHIVE_EXT=7z
else
  PROJ_ARCHIVE_EXT=tar.gz
  E2E_ARCHIVE_EXT=tar.gz
fi

REF=${REF:=$(git rev-parse HEAD)}
PROJ_VERSION=${REF}-${Target}_${Architecture}_${Linkage}

if [ "$Target" = 'linux' ]; then

  #
  # Linux packaging
  #

  cp -av $($SCRIPTS/gather.sh $BASEDIR/asdf/libs/linux64/*.so* /usr/lib64) $BASEDIR/asdf/libs/linux64

  . $BASEDIR/.venv/bin/activate
  pip install 'poetry==1.4.1'

  pushd $BASEDIR/jenkins
  poetry install

  poetry run python "$BASEDIR/jenkins/make_package.py" linux ${PROJ_VERSION} ${PROJ_ARCHIVE_EXT} ${E2E_ARCHIVE_EXT}

  ls

  popd

elif [ "$Target" = 'windows' ]; then

  #
  # Windows packaging
  #

  cp -av $DEPS/bin/*.{dll,exe} $BASEDIR/asdf/libs/win64
  cp -av $($SCRIPTS/gather.sh $BASEDIR/asdf/libs/win64/*.dll $BASEDIR/asdf/libs/win64/*.exe $MINGW_ROOT/bin) $BASEDIR/asdf/libs/win64

  pushd $DEPS/lib/python
  PYTHON_MODULES="fsrip.py hasher.py lightgrep.py ntfs_linker_copy.py"
  cp -av $PYTHON_MODULES $BASEDIR/asdf/libs/python
  popd
fi
