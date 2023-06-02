Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "> This script will automatically setup WSL for NXZR..."

# FIXME: Make sure the resource kernel is in place and replace kernel=*
# Create a global `.wslconfig` to current user's folder.
Write-Host "> Creating a global `".wslconfig`" file"
$wsl_conf_content = @"
[wsl2]
kernel=c:\\Users\\plusb\\.wsl\\nxzr-bzImage
"@
$home_dir = [System.Environment]::ExpandEnvironmentVariables("%USERPROFILE%")
$wsl_conf_path = Join-Path $home_dir ".wslconfig"
if (Test-Path $wsl_conf_path) {
    Write-Host "> Existing `".wslconfig`" found, checking content integrity..."
    # Replace existing config only when actual content mismatches.
    $current_conf_content = Get-Content $wsl_conf_path -Raw
    if ($current_conf_content.Trim() -ne $wsl_conf_content.Trim()) {
        Write-Host "> Existing `".wslconfig`" does not match to the desired config, replacing it with new content after backing it up..."
        # Create a backup copy of `.wslconfig` as `wslconfig.back[.N]` if there's existing one
        $backup_path = Join-Path $home_dir "wslconfig.back"
        $index = 0
        while (Test-Path $backup_path) {
            $index++
            $backup_path = Join-Path $home_dir "wslconfig.back.$index"
        }
        Copy-Item $wsl_conf_path $backup_path -Force
        # Fill `.wslconfig` with new content
        $wsl_conf_content | Set-Content $wsl_conf_path -Force
    }
    else {
        Write-Host "> Existing `".wslconfig`" matchs to the desired content, ignoring..."
    }
}
else {
    Write-Host "> No existing `".wslconfig`" found, creating a new one"
    # If there's no file present, just create a new one
    $wsl_conf_content | Set-Content $wsl_conf_path -Force
}