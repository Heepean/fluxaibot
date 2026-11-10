$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$releaseName = "release-$(Get-Date -Format 'yyyyMMdd-HHmmss')"
$releaseDirectory = Join-Path $root "dist\$releaseName"
$portableSource = Join-Path $root 'target\release\local_ai_hub.exe'
$nsisDirectory = Join-Path $root 'target\release\bundle\nsis'
$appVersion = (Get-Content -LiteralPath (Join-Path $root 'src-tauri\tauri.conf.json') -Raw |
    ConvertFrom-Json).version
$installerName = "FluxAiBot_${appVersion}_x64-setup.exe"

if (Test-Path -LiteralPath $releaseDirectory) {
    throw "Release output already exists: $releaseDirectory"
}

cargo tauri --version *> $null
if ($LASTEXITCODE -ne 0) {
    throw 'Tauri CLI 2.12.4 is required. Install it with: cargo install tauri-cli --version 2.12.4 --locked'
}

New-Item -ItemType Directory -Path $releaseDirectory | Out-Null

cargo build --release --package local_ai_hub --bin local_ai_hub
if ($LASTEXITCODE -ne 0) {
    throw 'Portable application build failed.'
}
if (-not (Test-Path -LiteralPath $portableSource -PathType Leaf)) {
    throw "Portable application binary is missing: $portableSource"
}

Copy-Item -LiteralPath $portableSource `
    -Destination (Join-Path $releaseDirectory 'FluxAiBot-portable.exe')

cargo tauri build --bundles nsis
if ($LASTEXITCODE -ne 0) {
    throw 'NSIS installer build failed.'
}

$installer = Join-Path $nsisDirectory $installerName
if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) {
    throw "NSIS setup executable for version $appVersion is missing: $installer"
}

Copy-Item -LiteralPath $installer `
    -Destination (Join-Path $releaseDirectory 'FluxAiBot-setup.exe')

$checksumLines = @(
    Get-ChildItem -LiteralPath $releaseDirectory -File |
        Sort-Object Name |
        ForEach-Object {
            $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            "$hash  $($_.Name)"
        }
)
Set-Content -LiteralPath (Join-Path $releaseDirectory 'SHA256SUMS.txt') `
    -Value $checksumLines -Encoding ascii

Get-ChildItem -LiteralPath $releaseDirectory -File |
    ForEach-Object {
        $hash = Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256
        [pscustomobject]@{
            File   = $_.Name
            Bytes  = $_.Length
            SHA256 = $hash.Hash
        }
    } |
    Format-Table -AutoSize

Write-Output "Release artifacts: $releaseDirectory"
