$env:RUST_LOG="info"
Set-Location $PSScriptRoot
../../../bftools/bftools.exe miz --output ./Caucasus.miz --base ./base.miz --weapon ./weapon.miz --warehouse ./warehouse.miz --options ./options.miz
