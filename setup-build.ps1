# Why?
# In order to safely interface with DCS we need to link to the same version of the lua runtime
# that DCS is using. If we link to a different version, very bad things like structures not being compatible,
# globals being in the wrong place, or in two places, the GC of one version running on the heap of the other
# version, etc might happen. There's a huge list, and you just don't want to go there.
#
# Setting the environment variables below tells the mlua build scripts that it should produce a dll linked
# dynamically to the lua runtime, meaning it will not include one of it's own, it will just assume that
# when our dll is loaded, lua is already linked, which DCS will ensure is true, because it loads lua.dll
# very early on.
#
# In order to link dynamically to lua, we need a .lib file that describes all the functions exported by the dll,
# so that the linker can check that the dynamic linkage will work at run time. That is why there is a lua.lib
# checked in at the root of the repo.
#
# When a new version of DCS is released, it *could* use a different version of lua (although ED has not upgraded
# their lua in aaaaaaages). As such every time DCS is updated we should copy lua.dll from the bin-mt folder of DCS
# to this directory and run dll2lib.bat on it to generate a new lua.lib file. 
# (you must have the windows sdk installed and the tools in your path for that to work).

$env:LUA_LIB=Get-Location
$env:LUA_LINK="dylib"
$env:LUA_LIB_NAME="lua"
