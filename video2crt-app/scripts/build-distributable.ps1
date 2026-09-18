[CmdletBinding()]
param(
    [switch]$SkipModels,
    [switch]$CleanRuntime,
    [switch]$SkipBuild,
    [string]$OutputDirectory = "dist-distributable"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot ".." )).Path
$RepoRoot = (Resolve-Path (Join-Path $ProjectRoot ".." )).Path
$PackagingRoot = Join-Path $ProjectRoot ".packaging"
$RuntimeRoot = Join-Path $PackagingRoot "runtime"
$DownloadsRoot = Join-Path $PackagingRoot "downloads"
$ToolsRoot = Join-Path $RuntimeRoot "tools"
$PythonRoot = Join-Path $ToolsRoot "python"
$AppRoot = Join-Path $RuntimeRoot "app"
$ModelsRoot = Join-Path $RuntimeRoot "models"
$OutputRoot = [IO.Path]::GetFullPath((Join-Path $ProjectRoot $OutputDirectory))

function Ensure-Directory([string]$Path) {
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Download-File([string]$Uri, [string]$Destination) {
    Ensure-Directory ([IO.Path]::GetDirectoryName($Destination))
    if ((Test-Path -LiteralPath $Destination) -and (Get-Item -LiteralPath $Destination).Length -gt 0) {
        Write-Host "[CACHE] $([IO.Path]::GetFileName($Destination))"
        return
    }
    Write-Host "[GET] $Uri"
    Invoke-WebRequest -Uri $Uri -OutFile $Destination -UseBasicParsing
}

function Expand-ArchiveTo([string]$Archive, [string]$Destination) {
    if (Test-Path -LiteralPath $Destination) {
        Remove-Item -LiteralPath $Destination -Recurse -Force
    }
    Ensure-Directory $Destination
    Expand-Archive -LiteralPath $Archive -DestinationPath $Destination -Force
}

function Copy-DirectoryContents([string]$Source, [string]$Destination) {
    Ensure-Directory $Destination
    Get-ChildItem -LiteralPath $Source -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $Destination -Recurse -Force
    }
}

function Find-HostPython {
    $candidates = @(
        (Join-Path $env:LOCALAPPDATA "Programs/Python/Python312/python.exe"),
        (Get-Command python -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -ErrorAction SilentlyContinue)
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) }
    foreach ($candidate in $candidates) {
        $version = & $candidate --version 2>&1
        if ($version -match "Python 3\.12\.") {
            return $candidate
        }
    }
    throw "需要 Python 3.12 主機環境來建立可攜式 runtime。"
}

if ($CleanRuntime) {
    foreach ($path in @($RuntimeRoot, $DownloadsRoot)) {
        $resolved = [IO.Path]::GetFullPath($path)
        if ($resolved.StartsWith([IO.Path]::GetFullPath($PackagingRoot), [StringComparison]::OrdinalIgnoreCase)) {
            if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
        }
    }
}

Ensure-Directory $DownloadsRoot
Ensure-Directory $ToolsRoot
Ensure-Directory $AppRoot
Ensure-Directory $ModelsRoot

$pythonZip = Join-Path $DownloadsRoot "python-3.12.10-embed-amd64.zip"
Download-File "https://www.python.org/ftp/python/3.12.10/python-3.12.10-embed-amd64.zip" $pythonZip
$pythonExtract = Join-Path $PackagingRoot "python-extract"
Expand-ArchiveTo $pythonZip $pythonExtract
Copy-DirectoryContents $pythonExtract $PythonRoot
$pth = Get-ChildItem -LiteralPath $PythonRoot -Filter "python312._pth" -File | Select-Object -First 1
if (-not $pth) { throw "Python embeddable package 缺少 python312._pth。" }
$pthLines = @(
    "python312.zip",
    ".",
    "Lib/site-packages",
    "app",
    "",
    "import site"
)
Set-Content -LiteralPath $pth.FullName -Value $pthLines -Encoding ascii

$hostPython = Find-HostPython
$sitePackages = Join-Path $PythonRoot "Lib/site-packages"
Ensure-Directory $sitePackages
& $hostPython -m pip install --disable-pip-version-check --no-compile --only-binary=:all: --upgrade --target $sitePackages -r (Join-Path $ProjectRoot "scripts/runtime-requirements.txt")
if ($LASTEXITCODE -ne 0) { throw "Python runtime 依賴安裝失敗。" }

$ytDlp = Join-Path $ToolsRoot "yt-dlp.exe"
Download-File "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe" $ytDlp

$nodeZip = Join-Path $DownloadsRoot "node-v22.23.2-win-x64.zip"
Download-File "https://nodejs.org/dist/v22.23.2/node-v22.23.2-win-x64.zip" $nodeZip
$nodeExtract = Join-Path $PackagingRoot "node-extract"
Expand-ArchiveTo $nodeZip $nodeExtract
$nodeExe = Get-ChildItem -LiteralPath $nodeExtract -Filter "node.exe" -Recurse -File | Select-Object -First 1
if (-not $nodeExe) { throw "Node.js archive 缺少 node.exe。" }
Copy-Item -LiteralPath $nodeExe.FullName -Destination (Join-Path $ToolsRoot "node.exe") -Force

$ffmpegZip = Join-Path $DownloadsRoot "ffmpeg-release-essentials.zip"
Download-File "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip" $ffmpegZip
$ffmpegExtract = Join-Path $PackagingRoot "ffmpeg-extract"
Expand-ArchiveTo $ffmpegZip $ffmpegExtract
$ffmpegBin = Get-ChildItem -LiteralPath $ffmpegExtract -Directory -Recurse | Where-Object { Test-Path (Join-Path $_.FullName "bin/ffmpeg.exe") } | Select-Object -First 1
if (-not $ffmpegBin) { throw "FFmpeg archive 缺少 bin/ffmpeg.exe。" }
Copy-DirectoryContents (Join-Path $ffmpegBin.FullName "bin") $ToolsRoot

Copy-Item -LiteralPath (Join-Path $ProjectRoot "src-tauri/bin/pipeline_cli.py") -Destination $AppRoot -Force
Copy-Item -LiteralPath (Join-Path $ProjectRoot "src-tauri/bin/subtitle_engine.py") -Destination $AppRoot -Force
Copy-Item -LiteralPath (Join-Path $ProjectRoot "src-tauri/bin/vendor") -Destination $AppRoot -Recurse -Force
Copy-Item -LiteralPath (Join-Path $RepoRoot "src") -Destination $AppRoot -Recurse -Force
Copy-Item -LiteralPath (Join-Path $RepoRoot "lyrics_translations.json") -Destination $AppRoot -Force
Ensure-Directory (Join-Path $AppRoot "scripts")
Copy-Item -LiteralPath (Join-Path $ProjectRoot "scripts/crt.glsl") -Destination (Join-Path $AppRoot "scripts/crt.glsl") -Force

if (-not $SkipModels) {
    $env:VIDEO2CRT_MODEL_ROOT = $ModelsRoot
    # Use the staged Python so the exact wheel set used by the installer also
    # performs the snapshot download.
    & (Join-Path $PythonRoot "python.exe") (Join-Path $ProjectRoot "scripts/stage-models.py")
    if ($LASTEXITCODE -ne 0) { throw "模型 staging 失敗。" }
} else {
    Write-Warning "已略過模型；這次只會產生 runtime 預覽包，無法在離線狀態完成字幕翻譯。"
}

$required = @(
    (Join-Path $PythonRoot "python.exe"),
    (Join-Path $ToolsRoot "node.exe"),
    (Join-Path $ToolsRoot "yt-dlp.exe"),
    (Join-Path $ToolsRoot "ffmpeg.exe"),
    (Join-Path $ToolsRoot "ffprobe.exe"),
    (Join-Path $AppRoot "pipeline_cli.py"),
    (Join-Path $AppRoot "subtitle_engine.py"),
    (Join-Path $AppRoot "src/video2crt/asr.py"),
    (Join-Path $AppRoot "src/video2crt/subtitle.py"),
    (Join-Path $AppRoot "vendor/opencc/config/t2tw.json")
)
if (-not $SkipModels) {
    $required += @(
        (Join-Path $ModelsRoot "asr/tokenizer.json"),
        (Join-Path $ModelsRoot "asr/vocabulary.txt"),
        (Join-Path $ModelsRoot "translation/model.bin"),
        (Join-Path $ModelsRoot "translation/sentencepiece.bpe.model")
    )
}
$missing = @($required | Where-Object { -not (Test-Path -LiteralPath $_ -PathType Leaf) })
if ($missing.Count -gt 0) { throw "runtime 缺少必要檔案：`n$($missing -join "`n")" }

$manifest = [ordered]@{
    generatedAt = (Get-Date).ToUniversalTime().ToString("o")
    python = (& $hostPython --version 2>&1 | Out-String).Trim()
    node = (& (Join-Path $ToolsRoot "node.exe") --version 2>&1 | Out-String).Trim()
    ytDlp = (& $ytDlp --version 2>&1 | Out-String).Trim()
    ffmpeg = (& (Join-Path $ToolsRoot "ffmpeg.exe") -version 2>&1 | Select-Object -First 1).ToString().Trim()
    modelsIncluded = (-not $SkipModels)
}
$manifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $RuntimeRoot "runtime-manifest.json") -Encoding utf8

if (-not $SkipBuild) {
    Push-Location $ProjectRoot
    try {
        npm run tauri:build
        if ($LASTEXITCODE -ne 0) { throw "Tauri 安裝包建置失敗。" }
    } finally {
        Pop-Location
    }
    Ensure-Directory $OutputRoot
    $installers = @(Get-ChildItem -LiteralPath (Join-Path $ProjectRoot "src-tauri/target/release/bundle/nsis") -Filter "*.exe" -File)
    if ($installers.Count -eq 0) { throw "Tauri 沒有產生 NSIS 安裝程式。" }
    $installers | Copy-Item -Destination $OutputRoot -Force
    Write-Host "[OK] 安裝程式輸出：$OutputRoot"
} else {
    Write-Host "[OK] runtime staging 完成：$RuntimeRoot"
}
