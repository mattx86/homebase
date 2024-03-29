#!/bin/bash

VERSION=$(grep 'const VERSION' src/main.rs | egrep -o '"[^"]+"' | tr -d '"')
RELEASE_ARCH=$(uname -m)
RELEASE_OS=$(uname -o | egrep -o 'Linux' | tr '[A-Z]' '[a-z]')
RELEASE_DIR=homebase-$VERSION-$RELEASE_ARCH-$RELEASE_OS
RELEASE_TAR=$RELEASE_DIR.tar.gz

/bin/rm -rf $RELEASE_DIR
cargo build -r && \
  mkdir $RELEASE_DIR && \
  cp LICENSE $RELEASE_DIR/ && \
  cp README.md $RELEASE_DIR/ && \
  cp -a examples $RELEASE_DIR/ && \
  cp target/release/homebase $RELEASE_DIR/ && \
  cp install.sh $RELEASE_DIR/ && \
  tar -czv --owner=0 --group=0 -f $RELEASE_TAR $RELEASE_DIR

