$env:RUST_LOG="info"
Set-Location $PSScriptRoot
../../../../bftools/bftools.exe miz --output ./PG.miz --base ./base.miz --weapon ../weapon.miz --warehouse Warehouse/warehouse.miz --options ../options.miz
