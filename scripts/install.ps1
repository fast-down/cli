$ErrorActionPreference = 'Stop'

$ARCH = $env:PROCESSOR_ARCHITECTURE
$BASE_URL = "https://fast-down-update.s121.top/cli/download/latest"
$DOWNLOAD_URL = "$BASE_URL/windows/$ARCH"
$INSTALL_DIR = "$env:LOCALAPPDATA\Programs\fast-down-cli"
$BIN_NAME = "fd.exe"
$TMP_FILE = [System.IO.Path]::GetTempFileName()

Write-Host "✨ Downloading $DOWNLOAD_URL"
try
{
    Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $TMP_FILE -UseBasicParsing
} catch
{
    Write-Host "❌ Error: Failed to download the file: $_"
    Remove-Item -Path $TMP_FILE
    exit 1
}

if (-not (Test-Path -Path $INSTALL_DIR))
{
    New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null
}
Move-Item -Path $TMP_FILE -Destination "$INSTALL_DIR\$BIN_NAME" -Force
Write-Host "🎉 Installed to $INSTALL_DIR\$BIN_NAME"

$UserPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -split ';' -notcontains $INSTALL_DIR)
{
    Write-Host "🔧 Adding $INSTALL_DIR to User PATH"
    $NewPath = if ([string]::IsNullOrWhiteSpace($UserPath))
    {
        $INSTALL_DIR
    } else
    {
        "$UserPath;$INSTALL_DIR"
    }

    try
    {
        [System.Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
        $env:PATH += ";$INSTALL_DIR"
        Write-Host "🔔 Please restart your terminal (or VS Code) for the changes to take full effect"
    } catch
    {
        Write-Host "❌ Error: Failed to update environment variables: $_"
        Write-Host "🔔 Please manually add $INSTALL_DIR to your PATH"
    }
} else
{
    Write-Host "🔔 $INSTALL_DIR is already in your PATH"
}

Write-Host "🚀 Installation complete! You can now run '$BIN_NAME'"
