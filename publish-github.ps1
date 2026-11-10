$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$repository = 'Heepean/fluxaibot'
$remoteUrl = "https://github.com/$repository.git"
$assetNames = @(
    'FluxAiBot-portable.exe',
    'FluxAiBot-setup.exe',
    'SHA256SUMS.txt'
)
$appConfig = Get-Content -LiteralPath (Join-Path $root 'src-tauri\tauri.conf.json') -Raw |
    ConvertFrom-Json
$releaseVersion = $appConfig.version
$releaseTag = "v$releaseVersion"
$releaseNotes = Join-Path $root "docs\RELEASE_NOTES_$releaseTag.md"

function Invoke-Git {
    param([Parameter(Mandatory)][string[]]$Arguments)

    & git @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed with exit code $LASTEXITCODE."
    }
}

function Invoke-Gh {
    param([Parameter(Mandatory)][string[]]$Arguments)

    & gh @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "gh $($Arguments -join ' ') failed with exit code $LASTEXITCODE."
    }
}

function Get-Sha256Hex {
    param([Parameter(Mandatory)][string]$Path)

    $stream = [System.IO.File]::OpenRead($Path)
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        return [BitConverter]::ToString($sha256.ComputeHash($stream)).Replace('-', '').ToLowerInvariant()
    }
    finally {
        $sha256.Dispose()
        $stream.Dispose()
    }
}

Set-Location -LiteralPath $root

if (-not (Test-Path -LiteralPath (Join-Path $root '.git') -PathType Container)) {
    throw 'The local Git repository has not been initialized.'
}
if ((git status --porcelain)) {
    throw 'The working tree has uncommitted changes. Commit them before publishing.'
}
if (-not (git rev-parse --verify 'refs/heads/main' 2>$null)) {
    throw 'The main branch is missing.'
}
if (-not (Test-Path -LiteralPath $releaseNotes -PathType Leaf)) {
    throw "Release notes are missing: $releaseNotes"
}
$releaseDirectory = Get-ChildItem -LiteralPath (Join-Path $root 'dist') -Directory |
    Sort-Object LastWriteTime -Descending |
    Where-Object {
        $candidate = $_.FullName
        ($assetNames | Where-Object {
            -not (Test-Path -LiteralPath (Join-Path $candidate $_) -PathType Leaf)
        }).Count -eq 0
    } |
    Select-Object -First 1 -ExpandProperty FullName
if (-not $releaseDirectory) {
    throw 'No complete release directory found. Run build-release.ps1 before publishing.'
}
$releaseAssets = @($assetNames | ForEach-Object { Join-Path $releaseDirectory $_ })

$checksumFile = $releaseAssets[2]
foreach ($line in Get-Content -LiteralPath $checksumFile) {
    if ($line -notmatch '^([a-fA-F0-9]{64})  (.+)$') {
        throw "Invalid release checksum line: $line"
    }
    $assetPath = Join-Path $releaseDirectory $Matches[2]
    if (-not (Test-Path -LiteralPath $assetPath -PathType Leaf)) {
        throw "Checksum references a missing release asset: $assetPath"
    }
    $actualHash = Get-Sha256Hex -Path $assetPath
    if ($actualHash -ine $Matches[1]) {
        throw "SHA-256 verification failed for $assetPath"
    }
}

Write-Host "FluxAiBot $releaseTag release files and checksums are verified." -ForegroundColor Green

& gh auth status *> $null
if ($LASTEXITCODE -ne 0) {
    Write-Host 'Opening the official GitHub sign-in. Complete the confirmation in your browser.' -ForegroundColor Yellow
    Invoke-Gh -Arguments @(
        'auth', 'login',
        '--hostname', 'github.com',
        '--git-protocol', 'https',
        '--web',
        '--skip-ssh-key'
    )
}

$authenticatedUser = & gh api user --jq '.login'
if ($LASTEXITCODE -ne 0 -or $authenticatedUser -ne 'Heepean') {
    throw "Sign in to GitHub as Heepean before publishing (current account: $authenticatedUser)."
}

Invoke-Gh -Arguments @('auth', 'setup-git', '--hostname', 'github.com')

$remote = git remote get-url origin 2>$null
if ($LASTEXITCODE -ne 0) {
    Invoke-Git -Arguments @('remote', 'add', 'origin', $remoteUrl)
}
elseif ($remote -ne $remoteUrl -and $remote -ne 'git@github.com:Heepean/fluxaibot.git') {
    throw "The origin remote points to '$remote'; expected $remoteUrl. No changes were pushed."
}

Invoke-Git -Arguments @('lfs', 'install', '--local')
Invoke-Gh -Arguments @(
    'repo', 'edit', $repository,
    '--description', 'Run local GGUF AI models on Windows with CUDA, Vulkan and CPU fallback, Hugging Face model search, a loopback-only API, and a 10-language interface that follows Windows or your choice.',
    '--homepage', "https://github.com/$repository/releases/latest"
)

Write-Host 'Pushing the main branch and LFS-backed runtime files. This may take a while.' -ForegroundColor Cyan
Invoke-Git -Arguments @('push', '--set-upstream', 'origin', 'main')

$release = $null
& gh release view $releaseTag --repo $repository --json url,assets 2>$null | ForEach-Object {
    try {
        $release = $_ | ConvertFrom-Json
    }
    catch {
        throw "Cannot parse GitHub Release metadata: $_"
    }
}

if (-not $release) {
    if (-not (git rev-parse --verify "refs/tags/$releaseTag" 2>$null)) {
        Invoke-Git -Arguments @('tag', '-a', $releaseTag, '-m', "FluxAiBot $releaseTag")
    }
    Invoke-Git -Arguments @('push', 'origin', "refs/tags/$releaseTag")
    Invoke-Gh -Arguments @(
        'release', 'create', $releaseTag,
        '--repo', $repository,
        '--title', "FluxAiBot $releaseTag",
        '--notes-file', $releaseNotes,
        '--latest'
    )
    $release = (& gh release view $releaseTag --repo $repository --json url,assets | ConvertFrom-Json)
}

$existingAssetNames = @($release.assets | ForEach-Object name)
$assetsToUpload = @($releaseAssets | Where-Object {
    $existingAssetNames -notcontains [System.IO.Path]::GetFileName($_)
})
if ($assetsToUpload.Count -gt 0) {
    Invoke-Gh -Arguments (@(
        'release', 'upload', $releaseTag,
        '--repo', $repository
    ) + $assetsToUpload)
}

$finalRelease = & gh release view $releaseTag --repo $repository --json url,assets
if ($LASTEXITCODE -ne 0) {
    throw 'Release was published, but its final verification failed.'
}
$finalRelease = $finalRelease | ConvertFrom-Json
$finalAssetNames = @($finalRelease.assets | ForEach-Object name)
foreach ($asset in $releaseAssets) {
    $name = [System.IO.Path]::GetFileName($asset)
    if ($finalAssetNames -notcontains $name) {
        throw "Release verification failed: $name was not uploaded."
    }
}

Write-Host ''
Write-Host "GitHub repository: https://github.com/$repository" -ForegroundColor Green
Write-Host "GitHub release: $($finalRelease.url)" -ForegroundColor Green
