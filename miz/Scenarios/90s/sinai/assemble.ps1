$env:RUST_LOG="info"
Set-Location $PSScriptRoot
../../../../bftools/bftools.exe miz --output ./Sinai90s.miz --base ./base.miz --weapon Options/weapon.miz --warehouse Options/warehouse.miz --options ../options.miz
