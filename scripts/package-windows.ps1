param(
    [string]$FfmpegPath
)

$ErrorActionPreference = 'Stop'
$repository = Split-Path -Parent $PSScriptRoot
$application = Join-Path $repository 'target\release\ayrex.exe'
if (-not (Test-Path -LiteralPath $application -PathType Leaf)) {
    throw 'Release executable not found. Run cargo build --release first.'
}

if (-not $FfmpegPath) {
    $ffmpegCommand = Get-Command ffmpeg.exe -ErrorAction SilentlyContinue
    if (-not $ffmpegCommand) {
        throw 'Pass -FfmpegPath with a portable FFmpeg executable.'
    }
    $FfmpegPath = $ffmpegCommand.Source
}

$FfmpegPath = (Resolve-Path -LiteralPath $FfmpegPath).Path
$encoders = & $FfmpegPath -hide_banner -encoders 2>&1 | Out-String
if ($LASTEXITCODE -ne 0 -or $encoders -notmatch '\blibx264\b') {
    throw 'The bundled FFmpeg must run and include the H.264 libx264 encoder.'
}

$distribution = Join-Path $repository 'dist\AyRex-Windows-x64'
New-Item -ItemType Directory -Force -Path $distribution | Out-Null
Copy-Item -LiteralPath $application -Destination (Join-Path $distribution 'AyRex.exe') -Force
Copy-Item -LiteralPath $FfmpegPath -Destination (Join-Path $distribution 'ffmpeg.exe') -Force
Copy-Item -LiteralPath (Join-Path $repository 'README.md') -Destination $distribution -Force
Copy-Item -LiteralPath (Join-Path $repository 'THIRD_PARTY_NOTICES.txt') -Destination $distribution -Force
Copy-Item -LiteralPath (Join-Path $repository 'licenses\FFmpeg-GPLv3.txt') -Destination $distribution -Force
$ffmpegVersion = & $FfmpegPath -version 2>&1 | Select-Object -First 1
if ($LASTEXITCODE -ne 0) {
    throw 'Could not determine the bundled FFmpeg version.'
}
Set-Content -LiteralPath (Join-Path $distribution 'FFMPEG-VERSION.txt') -Value $ffmpegVersion -Encoding utf8

$archive = Join-Path $repository 'dist\AyRex-Windows-x64.zip'
Compress-Archive -Path (Join-Path $distribution '*') -DestinationPath $archive -CompressionLevel Optimal -Force
Get-Item -LiteralPath $archive | Select-Object FullName, Length
