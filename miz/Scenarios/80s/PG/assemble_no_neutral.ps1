$env:RUST_LOG="info"
Set-Location $PSScriptRoot
../../../../bftools/bftools.exe miz --output ./PG_no_neutral.miz --base ./basenoneutral.miz --weapon ../weapon.miz --warehouse ../warehouse.miz --options ../options.miz
