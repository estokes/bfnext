# set this to your dcs bin-mt directory
$env:LUA_LIB="C:\\Program Files\\Eagle Dynamics\\DCS World\\bin-mt"
$env:LUA_LINK="dylib"

# we have an open issue with mlua team to figure out why this doesn't work as expected
# at the moment setting it causes us to try to link to the static lua lib, 
# which doesn't exist. E.D. calls their lua library lua.dll, even though it's
# lua 5.1. The canonical name for that is lua51.dll. We are supposed to be be
# able to set the LUA_LIB_NAME environment variable to anything we want, however
# if we do, the build will currently fail to link.

# $env:LUA_LIB_NAME="lua"

# we have to make the below symlink to work around LUA_LIB_NAME not working
# you will have to run this as administrator in your dcs bin-mt folder once
# before using bflib.dll

# new-item -itemtype SymbolicLink -Path "lua51.dll" -Value 'lua.dll'