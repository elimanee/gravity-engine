# Installs libopenmpt with vcpkg and points the build at it:
#   OPENMPT_LIB_DIR  folder with the import library, renamed openmpt.lib
#                    (the name openmpt-sys links against)
#   PATH             gets the DLLs, so tests and the game can run
$ErrorActionPreference = "Stop"
$root = Join-Path $env:VCPKG_INSTALLATION_ROOT "installed\x64-windows"
if (-not (Test-Path (Join-Path $root "bin\*openmpt*.dll"))) {
    & "$env:VCPKG_INSTALLATION_ROOT\vcpkg.exe" install libopenmpt:x64-windows
    if ($LASTEXITCODE -ne 0) { throw "vcpkg install failed" }
}
$lib = @("libopenmpt.lib", "openmpt.lib") | ForEach-Object { Join-Path $root "lib\$_" } | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $lib) {
    Get-ChildItem (Join-Path $root "lib") | Format-Table Name
    throw "libopenmpt import library not found"
}
$dir = Join-Path $env:RUNNER_TEMP "openmpt-lib"
New-Item -ItemType Directory -Force -Path $dir | Out-Null
Copy-Item $lib (Join-Path $dir "openmpt.lib")
"OPENMPT_LIB_DIR=$dir" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8
"OPENMPT_BIN_DIR=$(Join-Path $root 'bin')" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8
(Join-Path $root "bin") | Out-File -FilePath $env:GITHUB_PATH -Append -Encoding utf8
Get-ChildItem (Join-Path $root "bin") -Filter *.dll | Format-Table Name
