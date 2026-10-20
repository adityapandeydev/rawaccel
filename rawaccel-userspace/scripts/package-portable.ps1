param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$projectDirectory = Split-Path -Parent $PSScriptRoot
Push-Location $projectDirectory
try {
    if (-not $SkipBuild) {
        & npm run tauri -- build --no-bundle -- --locked
        if ($LASTEXITCODE -ne 0) { throw 'Portable application build failed.' }
    }
    $configuration = Get-Content -Raw -LiteralPath 'src-tauri/tauri.conf.json' | ConvertFrom-Json
    $binary = Join-Path $projectDirectory 'src-tauri/target/release/RawAccel.exe'
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw 'Build RawAccel.exe before packaging.' }
    $image = [System.IO.File]::ReadAllBytes($binary)
    $peOffset = [BitConverter]::ToInt32($image, 0x3c)
    if ([BitConverter]::ToUInt32($image, $peOffset) -ne 0x00004550 -or
        [BitConverter]::ToUInt16($image, $peOffset + 4) -ne 0x8664) {
        throw 'This portable package requires a Windows x64 executable.'
    }
    $outputDirectory = Join-Path $projectDirectory 'src-tauri/target/distribution'
    $stage = Join-Path $outputDirectory ('portable-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    Copy-Item -LiteralPath $binary -Destination (Join-Path $stage 'RawAccel.exe')
    Copy-Item -LiteralPath (Join-Path $projectDirectory '../LICENSE') -Destination (Join-Path $stage 'LICENSE.txt')
    Copy-Item -LiteralPath (Join-Path $projectDirectory 'RELEASE.txt') -Destination (Join-Path $stage 'README.txt')
    Set-Content -LiteralPath (Join-Path $stage 'portable.txt') -Encoding UTF8 -Value 'RawAccel portable mode: keep this file beside RawAccel.exe to store authoritative settings here.'
    $archive = Join-Path $outputDirectory ('RawAccel_' + $configuration.version + '_x64_portable.zip')
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $archive -Force
    $hasher = [Security.Cryptography.SHA256]::Create()
    $stream = [IO.File]::OpenRead($archive)
    try {
        $digest = [BitConverter]::ToString($hasher.ComputeHash($stream)).Replace('-', '')
        Write-Output ('SHA256 ' + $digest)
    } finally {
        $stream.Dispose()
        $hasher.Dispose()
    }
    Write-Output $archive
} finally {
    Pop-Location
}
