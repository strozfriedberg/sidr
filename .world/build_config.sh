Target=$1
Architecture=$2
Linkage=$3

INSTALL=${4:-$(realpath install)}
DEPS=${5:-$(realpath install)}

SCRIPTS=$(dirname $(realpath "$BASH_SOURCE"))
PROCS=$(nproc)

case "$Target" in
windows)
  EXE_EXT=.exe

  case "$Architecture" in
  32)
    CONFIGURE=mingw32-configure
    MAKE=mingw32-make
    CMAKE=mingw32-cmake
    STRIP=i686-w64-mingw32-strip
    MINGW_ROOT=/usr/i686-w64-mingw32/sys-root/mingw
    WINEARCH=win32
    WINEPREFIX=~/.wine32
    ;;
  64)
    CONFIGURE=mingw64-configure
    MAKE=mingw64-make
    CMAKE=mingw64-cmake
    STRIP=x86_64-w64-mingw32-strip
    MINGW_ROOT=/usr/x86_64-w64-mingw32/sys-root/mingw
    WINEARCH=win64
    WINEPREFIX=~/.wine64
    ;;
  esac

  WINEPATH=$MINGW_ROOT/bin
  export WINEARCH WINEPREFIX WINEPATH
  ;;
*)
  CONFIGURE=./configure
  EXE_EXT=''
  MAKE=make
  CMAKE=cmake
  STRIP=strip
  ;;
esac

case "$Linkage" in
shared)
  EXE_DOT_LIBS=.libs
  LINKAGE_FLAGS='--enable-shared --disable-static'
  ;;
shared-fat)
  LINKAGE_FLAGS='--enable-shared --enable-shared-fat --disable-static'
  ;;
static)
  EXE_DOT_LIBS=''
  LINKAGE_FLAGS='--disable-shared --enable-static'
  ;;
esac

if [ "$Target" = 'linux' -a "$Linkage" = 'shared' ]; then
  LD_LIBRARY_PATH=$DEPS/lib
  export LD_LIBRARY_PATH
fi

PKG_CONFIG_PATH=$DEPS/lib/pkgconfig

CONF_FLAGS="-C --prefix=$INSTALL --exec-prefix=$INSTALL --bindir=$INSTALL/bin --sbindir=$INSTALL/sbin --libexecdir=$INSTALL/libexec --sysconfdir=$INSTALL/etc --sharedstatedir=$INSTALL/com --localstatedir=$INSTALL/var --libdir=$INSTALL/lib --includedir=$INSTALL/include --datarootdir=$INSTALL/share --datadir=$INSTALL/share --infodir=$INSTALL/info --localedir=$INSTALL/locale --mandir=$INSTALL/man"

if [ "$Target" = 'macos' ]; then
  CONF_FLAGS+=" --with-boost=$(brew --prefix boost)"
fi

MAKE_FLAGS="-j$PROCS prefix=$INSTALL exec_prefix=$INSTALL bindir=$INSTALL/bin sbindir=$INSTALL/sbin sysconfdir=$INSTALL/etc datadir=$INSTALL/share includedir=$INSTALL/include libdir=$INSTALL/lib libexecdir=$INSTALL/libexec localstatedir=$INSTALL/var sharedstatedir=$INSTALL/com mandir=$INSTALL/man infodir=$INSTALL/info V=1 VERBOSE=1"

CPPFLAGS="-I$DEPS/include"
LDFLAGS="-L$DEPS/lib"

if [ "$Target" = 'macos' -a "$Linkage" = 'shared' ]; then
  # OpenSSL is not automatically linked by homebrew, so we will explicitly reference it
  LDFLAGS="$LDFLAGS -L$(brew --prefix)/lib  -L$(brew --prefix openssl)/lib -Wl,-rpath,$DEPS/lib"
  CPPFLAGS="$CPPFLAGS -I$(brew --prefix openssl)/include -I$(brew --prefix)/include"
  PKG_CONFIG_PATH="$PKG_CONFIG_PATH:$(brew --prefix openssl)/lib/pkgconfig"
fi

is_installed() {
  INSTALLER="your package manager"
  _LDFLAGS=""
  if [ $Target = 'macos' ]; then
    _LDFLAGS="-L$(brew --prefix)/lib  -L$(brew --prefix openssl)/lib"
    INSTALLER="macports or homebrew"
  fi

  # if gcc -l can't find the library, it'll say so; if it can,
  # it will complain about undefined symbols since there's nothing to link
  if [ $(gcc $_LDFLAGS -l$1 2>&1 | grep -ciE "undefined (symbol|reference)") = 0 ]; then
    echo "lib$1 wasn't found and should be installed via $INSTALLER"
  fi
}

configure_it() {
  $CONFIGURE CFLAGS="$CFLAGS" CPPFLAGS="$CPPFLAGS" LDFLAGS="$LDFLAGS" PKG_CONFIG_PATH=$PKG_CONFIG_PATH $LINKAGE_FLAGS $DEPS_FLAGS $CONF_FLAGS
}

make_it() {
  $MAKE $MAKE_FLAGS
}

make_check_it() {
  if [ $Target = 'windows' ]; then
    CHECK_TARGET=check
  else
    CHECK_TARGET=${CHECK_TARGET:-check}
  fi
  $MAKE $MAKE_FLAGS $CHECK_TARGET
}

make_install_it() {
  $MAKE $MAKE_FLAGS install
}

install_it() {
  make_install_it
}

make_clean_it() {
  $MAKE $MAKE_FLAGS clean
}

check_static() {
  $SCRIPTS/check_static.sh $@
}
