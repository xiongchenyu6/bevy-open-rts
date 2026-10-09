# Package an existing native MSVC build; never replaces an earlier artifact.
[CmdletBinding()]
param(
    [string]$Binary = "",
    [string]$Output = ""
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$Root = Split-Path -Parent $PSScriptRoot
if (!$Binary) { $Binary = Join-Path $Root 'target/x86_64-pc-windows-msvc/release/bevy-open-rts.exe' }
if (!$Output) { $Output = Join-Path $Root 'dist' }
$Binary = (Resolve-Path -LiteralPath $Binary).Path
$Output = [System.IO.Path]::GetFullPath($Output)
$Name = 'bevy-open-rts-windows-x86_64'
$Stage = Join-Path $Output $Name
$Archive = "$Stage.zip"
foreach ($Path in @($Stage, $Archive, "$Archive.sha256")) {
    if (Test-Path -LiteralPath $Path) { throw "Refusing to replace existing artifact: $Path" }
}
# Verify PE signature and AMD64 machine type before copying any package files.
$Stream = [System.IO.File]::OpenRead($Binary)
$Reader = [System.IO.BinaryReader]::new($Stream)
try {
    if ($Reader.ReadUInt16() -ne 0x5A4D) { throw 'Expected a Windows PE executable' }
    $Stream.Position = 0x3C
    $Offset = $Reader.ReadUInt32()
    $Stream.Position = $Offset
    if ($Reader.ReadUInt32() -ne 0x00004550 -or $Reader.ReadUInt16() -ne 0x8664) {
        throw 'Expected an AMD64 Windows PE executable'
    }
} finally { $Reader.Dispose(); $Stream.Dispose() }
New-Item -ItemType Directory -Path $Stage -Force | Out-Null
Copy-Item -LiteralPath $Binary -Destination (Join-Path $Stage 'bevy-open-rts.exe')
Copy-Item -LiteralPath (Join-Path $Root 'assets') -Destination $Stage -Recurse
# Full-font sources are local subset-generation inputs, not game runtime assets.
Get-ChildItem -LiteralPath (Join-Path $Stage 'assets') -Filter '*.full.ttf' -Recurse -File | Remove-Item
Copy-Item -LiteralPath (Join-Path $Root 'docs/PLAYER_GUIDE.md') -Destination (Join-Path $Stage 'README.md')
Copy-Item -LiteralPath (Join-Path $Root 'docs/WINDOWS_DELIVERY.md') -Destination (Join-Path $Stage 'BUILD_NOTES.md')
foreach ($Notice in @('ASSET_PROVENANCE.md', 'asset-source-hashes.json')) {
    Copy-Item -LiteralPath (Join-Path $Root "docs/$Notice") -Destination (Join-Path $Stage $Notice)
}
Copy-Item -LiteralPath (Join-Path $Root 'ASSETS.md') -Destination (Join-Path $Stage 'ASSETS.md')
Get-ChildItem -LiteralPath $Root -Filter 'LICENSE*' -File | Copy-Item -Destination $Stage
# The install can be read-only. Save/config paths are also independently
# resolved by the game from LOCALAPPDATA, rather than from this working directory.
$Launcher = @'
@echo off
setlocal
if not defined LOCALAPPDATA (
  echo Windows did not provide LOCALAPPDATA. Cannot locate user data.
  exit /b 1
)
set "BEVY_ASSET_ROOT=%~dp0"
set "GAME_DATA=%LOCALAPPDATA%\bevy-open-rts"
if not exist "%GAME_DATA%" mkdir "%GAME_DATA%"
if not exist "%GAME_DATA%" exit /b 1
cd /d "%GAME_DATA%"
if errorlevel 1 exit /b 1
"%~dp0bevy-open-rts.exe" %*
exit /b %errorlevel%
'@
[System.IO.File]::WriteAllText((Join-Path $Stage 'launch.cmd'), ($Launcher -replace "`r?`n", "`r`n"), [System.Text.Encoding]::ASCII)
$Commit = (& git -C $Root rev-parse HEAD)
if ($LASTEXITCODE -ne 0) { throw 'Could not record source revision' }
$AssetHashes = [ordered]@{}
Get-ChildItem -LiteralPath (Join-Path $Stage 'assets') -Recurse -File | Sort-Object FullName | ForEach-Object {
    $RelativePath = $_.FullName.Substring($Stage.Length + 1).Replace('\', '/')
    $AssetHashes[$RelativePath] = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
}
$Info = [ordered]@{
    runtime_asset_sha256 = $AssetHashes
    source_commit = $Commit
    source_worktree_dirty = [bool](& git -C $Root status --porcelain)
    binary_sha256 = (Get-FileHash -LiteralPath $Binary -Algorithm SHA256).Hash.ToLowerInvariant()
    target = 'x86_64-pc-windows-msvc'
    rust_version = (& rustc --version)
    validation = 'Packaging alone does not establish runtime or clean Windows compatibility.'
}
$Info | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $Stage 'BUILD_INFO.json') -Encoding utf8
Compress-Archive -LiteralPath $Stage -DestinationPath $Archive -CompressionLevel Optimal
$Checksum = (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
"$Checksum  $Name.zip" | Set-Content -LiteralPath "$Archive.sha256" -Encoding ascii
Write-Output "Archive: $Archive"
Write-Output "SHA256: $Checksum"
