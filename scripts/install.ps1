# Install the tracked mdv Windows binary from the main branch.
# Usage:
#   irm https://raw.githubusercontent.com/posaune0423/mdv/main/scripts/install.ps1 | iex
# Optional environment variables:
#   MDV_INSTALL_DIR  destination directory (default: $env:LOCALAPPDATA\mdv\bin)
#   MDV_BINARY_URL   HTTPS binary URL; HTTP is permitted only for localhost CI tests

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$defaultInstallDir = Join-Path $env:LOCALAPPDATA 'mdv\bin'
$installDir = if ([string]::IsNullOrWhiteSpace($env:MDV_INSTALL_DIR)) {
    $defaultInstallDir
} else {
    [System.IO.Path]::GetFullPath($env:MDV_INSTALL_DIR)
}
$binaryUrl = if ([string]::IsNullOrWhiteSpace($env:MDV_BINARY_URL)) {
    'https://raw.githubusercontent.com/posaune0423/mdv/main/bin/mdv.exe'
} else {
    $env:MDV_BINARY_URL
}

function Test-MdvPortableExecutable {
    param([Parameter(Mandatory = $true)][string]$Path)

    $stream = [System.IO.File]::OpenRead($Path)
    try {
        if ($stream.Length -lt 64) {
            return $false
        }

        $header = New-Object byte[] 64
        if ($stream.Read($header, 0, $header.Length) -ne $header.Length) {
            return $false
        }
        if ($header[0] -ne 0x4D -or $header[1] -ne 0x5A) {
            return $false
        }

        $peOffset = [System.BitConverter]::ToInt32($header, 0x3C)
        if ($peOffset -lt 64 -or $peOffset -gt ($stream.Length - 4)) {
            return $false
        }

        [void]$stream.Seek($peOffset, [System.IO.SeekOrigin]::Begin)
        $signature = New-Object byte[] 4
        if ($stream.Read($signature, 0, $signature.Length) -ne $signature.Length) {
            return $false
        }

        return $signature[0] -eq 0x50 -and $signature[1] -eq 0x45 -and $signature[2] -eq 0 -and $signature[3] -eq 0
    } finally {
        $stream.Dispose()
    }
}

try {
    $uri = [System.Uri]$binaryUrl
} catch {
    throw "MDV_BINARY_URL must be an absolute URL: $binaryUrl"
}
if ($uri.Scheme -ne 'https' -and -not ($uri.Scheme -eq 'http' -and $uri.IsLoopback)) {
    throw 'MDV_BINARY_URL must use HTTPS. HTTP is allowed only for a localhost test server.'
}

[System.IO.Directory]::CreateDirectory($installDir) | Out-Null
$stagedBinary = Join-Path $installDir ('.mdv-{0}.new.exe' -f $PID)
$targetBinary = Join-Path $installDir 'mdv.exe'

try {
    Invoke-WebRequest -Uri $uri -OutFile $stagedBinary -UseBasicParsing

    if (-not (Test-MdvPortableExecutable -Path $stagedBinary)) {
        throw 'Downloaded file is not a Windows PE executable; mdv was not installed.'
    }

    $versionOutput = & $stagedBinary --version 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Downloaded executable failed --version (exit code $LASTEXITCODE); mdv was not installed. $versionOutput"
    }

    if (Test-Path -LiteralPath $targetBinary) {
        [System.IO.File]::Replace($stagedBinary, $targetBinary, $null)
    } else {
        [System.IO.File]::Move($stagedBinary, $targetBinary)
    }
} finally {
    if (Test-Path -LiteralPath $stagedBinary) {
        Remove-Item -LiteralPath $stagedBinary -Force
    }
}

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$pathEntries = @()
if (-not [string]::IsNullOrWhiteSpace($userPath)) {
    $pathEntries = @($userPath -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}
$hasInstallDir = $false
foreach ($entry in $pathEntries) {
    $expandedEntry = [Environment]::ExpandEnvironmentVariables($entry)
    if ([string]::Equals([System.IO.Path]::GetFullPath($expandedEntry), $installDir, [System.StringComparison]::OrdinalIgnoreCase)) {
        $hasInstallDir = $true
        break
    }
}
if (-not $hasInstallDir) {
    $updatedUserPath = if ($pathEntries.Count -eq 0) { $installDir } else { "$userPath;$installDir" }
    [Environment]::SetEnvironmentVariable('Path', $updatedUserPath, 'User')
}

if (-not (($env:Path -split ';') | Where-Object { [string]::Equals($_, $installDir, [System.StringComparison]::OrdinalIgnoreCase) })) {
    $env:Path = "$installDir;$env:Path"
}

Write-Host "Installed mdv to $targetBinary"
Write-Host 'Open a new terminal to use the updated user PATH.'
