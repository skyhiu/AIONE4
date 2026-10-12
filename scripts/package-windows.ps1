# Builds the portable release zip: AIONE4-<version>-windows.zip with the exe and its notes.
# The web UI is embedded in the exe and WebView2's loader is linked statically, so the exe is
# the whole program; it needs the Microsoft Edge WebView2 Runtime (part of Windows 10/11).
param([Parameter(Mandatory = $true)][string]$Version)
$ErrorActionPreference = 'Stop'

$release = 'src-tauri/target/release'
$exe = @("$release/AIONE4.exe", "$release/aione4.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $exe) { throw "No exe found in $release" }

$stage = "release/AIONE4-$Version-windows"
if (Test-Path release) { Remove-Item release -Recurse -Force }
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Copy-Item $exe "$stage/AIONE4.exe"
Copy-Item 'docs/PORTABLE_README.txt' "$stage/README.txt"
Copy-Item 'docs/THIRD_PARTY_ASSETS.md' "$stage/THIRD_PARTY_ASSETS.md"
(Get-Content "$stage/README.txt") -replace '\{VERSION\}', $Version | Set-Content "$stage/README.txt"

$zip = "release/AIONE4-$Version-windows.zip"
Compress-Archive -Path "$stage/*" -DestinationPath $zip -CompressionLevel Optimal
$size = (Get-Item $zip).Length
Write-Host "Packaged $zip ($size bytes)"
Get-FileHash $zip -Algorithm SHA256 | Format-List
