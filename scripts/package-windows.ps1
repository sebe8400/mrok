$ErrorActionPreference = "Stop"

Set-Location (Split-Path $PSScriptRoot -Parent)

cargo build --release --locked

New-Item -ItemType Directory -Force -Path dist | Out-Null
Copy-Item target\release\mrok.exe dist\mrok.exe -Force
Compress-Archive -Path dist\mrok.exe -DestinationPath dist\mrok-windows-x86_64.zip -Force

Write-Host "Gotowe: dist\mrok-windows-x86_64.zip"
