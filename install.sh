#!/bin/bash

SUDO=""
[ $UID != 0 ] && SUDO="/bin/sudo"

[ -f target/release/homebase ] && HOMEBASE_SRC_PATH=target/release/homebase
[ -f homebase ] && HOMEBASE_SRC_PATH=homebase

$SUDO install -o root -g root -m 555 $HOMEBASE_SRC_PATH /usr/local/bin/
