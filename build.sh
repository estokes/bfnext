#! /bin/bash

export LUA_LIB="/c/Program Files/Eagle Dynamics/DCS World/bin"
export LUA_LIB_NAME="lua"
export LUA_LINK=dylib
cargo build $@
