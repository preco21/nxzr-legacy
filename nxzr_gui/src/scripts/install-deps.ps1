Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function New-TemporaryDirectory {
    $parent = [System.IO.Path]::GetTempPath()
    [string] $name = [System.Guid]::NewGuid()
    New-Item -ItemType Directory -Path (Join-Path $parent $name)
}

function Get-LatestGitHubReleaseBinary {
    [CmdletBinding()]
    param (
        [Parameter(Mandatory)]
        [string]$Repo,
        [Parameter(Mandatory)]
        [string]$Dir
    )
    $releases = "https://api.github.com/repos/$repo/releases"
    Write-Host "> Determining latest release from $Repo"
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $latest_release = (Invoke-WebRequest -Uri $releases -UseBasicParsing | ConvertFrom-Json)[0];
    $tag = $latest_release.tag_name
    $asset = $latest_release.assets[0]
    $asset_name = $asset.name
    $download_url = "https://github.com/$repo/releases/download/$tag/$asset_name"
    Write-Host "> Downloading binary from the latest release - $asset_name at $tag"
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $outpath = (Join-Path $Dir $asset_name)
    Invoke-WebRequest $download_url -Out $outpath
    return $outpath
}

function Install-Msi {
    [CmdletBinding()]
    param (
        [Parameter(Mandatory)]
        [string]$FilePath
    )
    $timestamp = get-date -Format yyyyMMddTHHmmss
    $log_file = '{0}-{1}.log' -f $FilePath, $timestamp
    $msi_arguments = @("/i", ('"{0}"' -f $FilePath), "/norestart", "/qn", "/L*v", $log_file)
    Start-Process "msiexec.exe" -ArgumentList $msi_arguments -Wait -NoNewWindow
}

function Test-CommandAvailable {
    [CmdletBinding()]
    param (
        [Parameter(Mandatory)]
        [string]$Command
    )
    return [bool](Get-Command -Name $Command -ErrorAction SilentlyContinue)
}

function Get-WindowsVersion {
    $display_version = (Get-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion" -Name DisplayVersion).DisplayVersion
    return [int]$display_version.SubString(0, 2)
}

# System compatibility checks.
$windows_version = Get-WindowsVersion
if ($windows_version -lt 22) {
    Write-Error "Your Windows version ($windows_version) is not compatible with NXZR: Please install the Windows Updates `"22H2`" or higher"
    Exit 1
}

if (!(Test-CommandAvailable -Command "winget")) {
    Write-Error "Unable to find the command `"winget`". Make sure to open Microsoft Store once to download required components."
    Exit 1
}

Write-Host "> This script will automatically install required dependencies of NXZR..."

# Create temporary directory to work with.
$tempdir = New-TemporaryDirectory
Write-Host "> Using temporary directory: $tempdir"

# Install WSL.
Write-Host "> Installing the `"Windows Subsystem for Linux (WSL)`""
Start-Process -FilePath "wsl.exe" -ArgumentList "--install --no-launch --web-download --no-distribution" -Wait -NoNewWindow
# Start-Process -FilePath "winget.exe" -ArgumentList "install --source msstore --disable-interactivity --accept-source-agreements --accept-package-agreements `"Windows Subsystem for Linux`"" -Wait -NoNewWindow

# Enable "Virtual Machine Platform" support.
Write-Host "> Enabling `"Virtual Machine Platform`" component"
Start-Process -FilePath "dism.exe" -ArgumentList "/online /enable-feature /featurename:VirtualMachinePlatform /all /norestart" -Wait -NoNewWindow

# Check if there's any updates from WSL.
Write-Host "> Checking for the `"Windows Subsystem for Linux (WSL)`" updates"
Start-Process -FilePath "wsl.exe" -ArgumentList "--update --web-download" -Wait -NoNewWindow

# Install `usbipd-win`.
Write-Host "> Installing `"usbipd-win`""
$usbipd_bin = Get-LatestGitHubReleaseBinary -Repo "dorssel/usbipd-win" -Dir $tempdir
Install-Msi -FilePath $usbipd_bin
