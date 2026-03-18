$ErrorActionPreference = 'Stop'

$ARCH = $env:PROCESSOR_ARCHITECTURE

switch -Regex ($ARCH) {
    "AMD64" { $ARCH_NAME = "x86_64" }
    "x86"   { $ARCH_NAME = "i686" }
    "ARM64" { $ARCH_NAME = "aarch64" }
    default {
        Write-Host "❌ Error: Unsupported Architecture: $ARCH"
        exit 1
    }
}

$BASE_URL = "https://fast-down-update.s121.top/cli/download/latest"
$DOWNLOAD_URL = "$BASE_URL/windows/$ARCH_NAME"

# Idiomatic Windows user-level installation path
$INSTALL_DIR = "$env:LOCALAPPDATA\Programs\fast-down"
$BIN_NAME = "fast-down.exe"
$TMP_FILE = [System.IO.Path]::GetTempFileName()

Write-Host "Downloading $DOWNLOAD_URL ..."
try {
    Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $TMP_FILE -UseBasicParsing
} catch {
    Write-Host "❌ Error: Failed to download the file."
    Remove-Item -Path $TMP_FILE -ErrorAction SilentlyContinue
    exit 1
}

if (-not (Test-Path -Path $INSTALL_DIR)) {
    New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null
}

Move-Item -Path $TMP_FILE -Destination "$INSTALL_DIR\$BIN_NAME" -Force

Write-Host "🎉 Installed to $INSTALL_DIR\$BIN_NAME"

# Check if the install directory is in the PATH
if (($env:PATH -split ';') -notcontains $INSTALL_DIR) {
    Write-Host "⚠️ Note: You need to add $INSTALL_DIR to your PATH environment variable."
    Write-Host "   You can do this by searching for 'Edit environment variables for your account' in the Start menu."
} else {
    Write-Host "🚀 You can now run 'fast-down' from your terminal!"
}
