# WPW Password Manager - Windows Installer
# Run: .\install-windows.ps1 [-ExtensionId <ID>] [-Uninstall]

param(
    [string]$ExtensionId = "",
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"

$InstallDir = "$env:LOCALAPPDATA\wpw"
$BinDir = $InstallDir
$NmManifest = "$InstallDir\nm-manifest.json"
$RegistryPath = "HKCU:\SOFTWARE\Google\Chrome\NativeMessagingHosts"
$EdgeRegistryPath = "HKCU:\SOFTWARE\Microsoft\Edge\NativeMessagingHosts"
$HostName = "com.wpw.host"

if ($Uninstall) {
    Write-Host "Uninstalling WPW..." -ForegroundColor Yellow
    
    # Remove registry entries
    Remove-Item -Path "$RegistryPath\$HostName" -ErrorAction SilentlyContinue
    Remove-Item -Path "$EdgeRegistryPath\$HostName" -ErrorAction SilentlyContinue
    
    # Remove from PATH
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath.Contains($BinDir)) {
        $newPath = ($currentPath -split ';' | Where-Object { $_ -ne $BinDir }) -join ';'
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    }
    
    # Remove files
    Remove-Item -Path $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
    
    Write-Host "WPW uninstalled successfully." -ForegroundColor Green
    exit 0
}

# Installation
Write-Host "Installing WPW Password Manager..." -ForegroundColor Cyan

# Create directory
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

# Copy binaries
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

if (Test-Path "$ProjectRoot\target\release\wpw.exe") {
    Copy-Item "$ProjectRoot\target\release\wpw.exe" "$BinDir\wpw.exe" -Force
    Copy-Item "$ProjectRoot\target\release\wpw-host.exe" "$BinDir\wpw-host.exe" -Force
    Write-Host "Copied binaries to $BinDir" -ForegroundColor Green
} else {
    Write-Host "Warning: Release binaries not found. Please build first with 'cargo build --release'" -ForegroundColor Yellow
}

# Add to PATH
$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $currentPath.Contains($BinDir)) {
    [Environment]::SetEnvironmentVariable("Path", "$currentPath;$BinDir", "User")
    Write-Host "Added $BinDir to PATH" -ForegroundColor Green
}

# Generate NM manifest
$allowedOrigins = @()
if ($ExtensionId) {
    $allowedOrigins += "chrome-extension://$ExtensionId/"
}

$manifest = @{
    name = $HostName
    description = "WPW Password Manager Native Host"
    path = "$BinDir\wpw-host.exe"
    type = "stdio"
    allowed_origins = $allowedOrigins
} | ConvertTo-Json

$manifest | Out-File -FilePath $NmManifest -Encoding utf8
Write-Host "Created NM manifest at $NmManifest" -ForegroundColor Green

# Register in Windows Registry
New-Item -Path "$RegistryPath\$HostName" -Force | Out-Null
Set-ItemProperty -Path "$RegistryPath\$HostName" -Name "(Default)" -Value $NmManifest

New-Item -Path "$EdgeRegistryPath\$HostName" -Force | Out-Null
Set-ItemProperty -Path "$EdgeRegistryPath\$HostName" -Name "(Default)" -Value $NmManifest

Write-Host "Registered Native Messaging Host for Chrome and Edge" -ForegroundColor Green

Write-Host ""
Write-Host "Installation complete!" -ForegroundColor Green
Write-Host "Please restart your terminal to use 'wpw' command."
Write-Host "Run 'wpw init' to create your vault."
