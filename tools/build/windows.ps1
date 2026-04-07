[CmdletBinding()]
param(
    [switch]$Clean,
    [switch]$SkipMsvcInstall,
    [ValidateSet("auto", "current-shell", "installed-msvc", "install-build-tools")]
    [string]$MsvcMode = "auto"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host "[LANScanner] $Message" -ForegroundColor Cyan
}

function Test-CommandAvailable {
    param([string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Test-MsvcCommandsAvailable {
    return (Test-CommandAvailable "cl.exe") -and (Test-CommandAvailable "link.exe")
}

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [string[]]$Arguments = @()
    )

    if ($Arguments.Count -gt 0) {
        & $FilePath @Arguments | Out-Host
    } else {
        & $FilePath | Out-Host
    }

    if ($LASTEXITCODE -ne 0) {
        throw "command failed ($LASTEXITCODE): $FilePath $($Arguments -join ' ')"
    }
}

function New-DirectoryIfMissing {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        New-Item -ItemType Directory -Path $Path | Out-Null
    }
}

function Read-NumericChoice {
    param(
        [Parameter(Mandatory = $true)][string]$Prompt,
        [Parameter(Mandatory = $true)][int[]]$AllowedChoices
    )

    $allowedSet = $AllowedChoices | Sort-Object -Unique
    while ($true) {
        $inputValue = (Read-Host $Prompt).Trim()
        $choice = 0
        if ([int]::TryParse($inputValue, [ref]$choice) -and $allowedSet -contains $choice) {
            return $choice
        }

        Write-Warning "Invalid selection. Choose one of: $($allowedSet -join ', ')"
    }
}

function Import-VsDevEnvironment {
    param([Parameter(Mandatory = $true)][string]$VsDevCmdPath)

    Write-Step "Importing MSVC build environment from $VsDevCmdPath"
    $env:VSCMD_SKIP_SENDTELEMETRY = "1"

    $dump = cmd.exe /d /s /c "call `"$VsDevCmdPath`" -no_logo -arch=x64 -host_arch=x64 >nul && set"
    if ($LASTEXITCODE -ne 0) {
        throw "failed to import Visual Studio developer environment"
    }

    foreach ($line in $dump) {
        $index = $line.IndexOf("=")
        if ($index -le 0) {
            continue
        }

        $name = $line.Substring(0, $index)
        $value = $line.Substring($index + 1)
        [System.Environment]::SetEnvironmentVariable($name, $value, "Process")
    }

    $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "Process")
}

function Get-VsWherePath {
    $candidates = @(
        $(if (${env:ProgramFiles(x86)}) { Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe" }),
        $(if ($env:ProgramFiles) { Join-Path $env:ProgramFiles "Microsoft Visual Studio\Installer\vswhere.exe" })
    )

    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate)) {
            return $candidate
        }
    }

    return $null
}

function Get-ExplicitVsDevCmdPath {
    $candidate = $env:LANSCANNER_VSDEVCMD
    if (-not $candidate) {
        return $null
    }

    if (-not (Test-Path -LiteralPath $candidate)) {
        throw "LANSCANNER_VSDEVCMD points to a missing file: $candidate"
    }

    return (Resolve-Path -LiteralPath $candidate).Path
}

function Get-VsInstallCandidates {
    $candidates = New-Object System.Collections.Generic.List[string]

    $vswhere = Get-VsWherePath
    if ($vswhere) {
        $discovered = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($LASTEXITCODE -eq 0 -and $discovered) {
            foreach ($path in $discovered) {
                $trimmed = $path.Trim()
                if ($trimmed) {
                    $candidates.Add($trimmed)
                }
            }
        }
    }

    $roots = @()
    foreach ($programFilesRoot in @(${env:ProgramFiles(x86)}, $env:ProgramFiles)) {
        if (-not $programFilesRoot) {
            continue
        }

        foreach ($year in @("2022", "2019")) {
            $roots += Join-Path $programFilesRoot "Microsoft Visual Studio\$year"
        }
    }

    foreach ($root in $roots) {
        if (-not $root -or -not (Test-Path -LiteralPath $root)) {
            continue
        }

        foreach ($edition in @("BuildTools", "Community", "Professional", "Enterprise")) {
            $candidate = Join-Path $root $edition
            if (Test-Path -LiteralPath $candidate) {
                $candidates.Add($candidate)
            }
        }
    }

    return $candidates | Select-Object -Unique
}

function Get-VsDevCmdPath {
    $explicitPath = Get-ExplicitVsDevCmdPath
    if ($explicitPath) {
        return $explicitPath
    }

    foreach ($installPath in Get-VsInstallCandidates) {
        $vsDevCmd = Join-Path $installPath "Common7\Tools\VsDevCmd.bat"
        if (Test-Path -LiteralPath $vsDevCmd) {
            return $vsDevCmd
        }
    }

    return $null
}

function Install-MsvcBuildTools {
    if (-not (Test-CommandAvailable "winget.exe")) {
        throw "MSVC Build Tools were not found and winget.exe is unavailable. Install Visual Studio 2022 Build Tools with the Desktop C++ workload and Windows SDK, then rerun this script."
    }

    Write-Warning "LANScanner requires system-level Microsoft C++ build tools for the Windows MSVC target."
    Write-Host "The script is about to install these system components via winget:" -ForegroundColor Yellow
    Write-Host "  - Visual Studio 2022 Build Tools" -ForegroundColor Yellow
    Write-Host "  - Desktop C++ workload (MSVC toolchain)" -ForegroundColor Yellow
    Write-Host "  - Windows 11 SDK 22621" -ForegroundColor Yellow

    $confirmation = Read-Host "Continue with system-level installation? Type 'yes' to continue"
    if ($confirmation.Trim().ToLowerInvariant() -ne "yes") {
        throw "MSVC installation cancelled by user."
    }

    Write-Step "MSVC Build Tools not found. Installing via winget after explicit confirmation."
    Invoke-Checked -FilePath "winget.exe" -Arguments @(
        "install",
        "--id", "Microsoft.VisualStudio.2022.BuildTools",
        "--exact",
        "--accept-source-agreements",
        "--accept-package-agreements",
        "--override", "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.22621"
    )
}

function Select-MsvcStrategy {
    param(
        [bool]$CurrentShellReady,
        [bool]$InstalledMsvcReady,
        [switch]$SkipInstall,
        [Parameter(Mandatory = $true)][string]$RequestedMode
    )

    if ($RequestedMode -ne "auto") {
        return $RequestedMode
    }

    if ($CurrentShellReady -and $InstalledMsvcReady) {
        Write-Host ""
        Write-Host "Detected multiple MSVC build paths:" -ForegroundColor Yellow
        Write-Host "  1. Use the current shell MSVC environment (for example VS Code terminal / Developer PowerShell)"
        Write-Host "  2. Import an installed Visual Studio / Build Tools environment"
        if (-not $SkipInstall) {
            Write-Host "  3. Install or repair Visual Studio Build Tools via winget"
        }

        $choices = @(1, 2)
        if (-not $SkipInstall) {
            $choices += 3
        }

        switch (Read-NumericChoice -Prompt "Select MSVC setup" -AllowedChoices $choices) {
            1 { return "current-shell" }
            2 { return "installed-msvc" }
            3 { return "install-build-tools" }
        }
    }

    if ($InstalledMsvcReady -and -not $CurrentShellReady) {
        Write-Host ""
        Write-Host "Detected an installed Visual Studio / Build Tools environment:" -ForegroundColor Yellow
        Write-Host "  1. Import the installed MSVC environment"
        if (-not $SkipInstall) {
            Write-Host "  2. Install or repair Visual Studio Build Tools via winget"
        }

        $choices = @(1)
        if (-not $SkipInstall) {
            $choices += 2
        }

        switch (Read-NumericChoice -Prompt "Select MSVC setup" -AllowedChoices $choices) {
            1 { return "installed-msvc" }
            2 { return "install-build-tools" }
        }
    }

    if ($CurrentShellReady) {
        return "current-shell"
    }

    if ($SkipInstall) {
        throw "MSVC Build Tools were not found. Install Visual Studio Build Tools, open a Developer shell, or rerun with -MsvcMode installed-msvc after setting LANSCANNER_VSDEVCMD."
    }

    Write-Host ""
    Write-Host "No ready-to-use MSVC environment was detected." -ForegroundColor Yellow
    Write-Host "  1. Install Visual Studio Build Tools via winget"
    Write-Host "  2. Cancel"

    switch (Read-NumericChoice -Prompt "Select MSVC setup" -AllowedChoices @(1, 2)) {
        1 { return "install-build-tools" }
        2 { throw "Build cancelled by user." }
    }
}

function Ensure-MsvcEnvironment {
    $currentShellReady = Test-MsvcCommandsAvailable
    $vsDevCmd = Get-VsDevCmdPath
    $selectedStrategy = Select-MsvcStrategy `
        -CurrentShellReady:$currentShellReady `
        -InstalledMsvcReady:($null -ne $vsDevCmd) `
        -SkipInstall:$SkipMsvcInstall `
        -RequestedMode $MsvcMode

    switch ($selectedStrategy) {
        "current-shell" {
            if (-not $currentShellReady) {
                throw "The current shell does not expose cl.exe/link.exe. Open a Developer shell first or choose another MSVC setup option."
            }

            Write-Step "Using existing MSVC environment from current shell"
        }
        "installed-msvc" {
            if (-not $vsDevCmd) {
                throw "No installed Visual Studio / Build Tools environment was detected. Install Build Tools, set LANSCANNER_VSDEVCMD, or choose install-build-tools."
            }

            Import-VsDevEnvironment -VsDevCmdPath $vsDevCmd
        }
        "install-build-tools" {
            if ($SkipMsvcInstall) {
                throw "Build Tools installation was disabled by -SkipMsvcInstall."
            }

            Install-MsvcBuildTools
            $vsDevCmd = Get-VsDevCmdPath
            if (-not $vsDevCmd) {
                throw "Unable to locate VsDevCmd.bat after the MSVC installation step. Verify that Visual Studio Build Tools and the Desktop C++ workload were installed successfully, then rerun this script."
            }

            Import-VsDevEnvironment -VsDevCmdPath $vsDevCmd
        }
        default {
            throw "Unsupported MSVC setup mode: $selectedStrategy"
        }
    }

    if (-not (Test-MsvcCommandsAvailable)) {
        $pathPreview = (($env:PATH -split ';') | Select-Object -First 8) -join ';'
        throw "MSVC environment setup completed, but cl.exe/link.exe are still unavailable. PATH preview: $pathPreview"
    }
}

function Ensure-LocalRustToolchain {
    param(
        [Parameter(Mandatory = $true)][string]$CargoHome,
        [Parameter(Mandatory = $true)][string]$RustupHome,
        [Parameter(Mandatory = $true)][string]$DownloadsDir
    )

    $cargoExe = Join-Path $CargoHome "bin\cargo.exe"
    $rustupExe = Join-Path $CargoHome "bin\rustup.exe"

    $env:RUSTUP_HOME = $RustupHome
    $env:CARGO_HOME = $CargoHome
    $env:PATH = "$($CargoHome)\bin;$env:PATH"

    if (-not (Test-Path -LiteralPath $rustupExe)) {
        $installerPath = Join-Path $DownloadsDir "rustup-init.exe"
        if (-not (Test-Path -LiteralPath $installerPath)) {
            Write-Step "Downloading rustup-init.exe into project-local tools cache"
            Invoke-WebRequest -Uri "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe" -OutFile $installerPath
        }

        Write-Step "Installing project-local Rust toolchain"
        Invoke-Checked -FilePath $installerPath -Arguments @(
            "-y",
            "--no-modify-path",
            "--profile", "minimal",
            "--default-toolchain", "stable-x86_64-pc-windows-msvc"
        )
    }

    if (-not (Test-Path -LiteralPath $cargoExe)) {
        throw "cargo.exe was not installed into the project-local toolchain directory."
    }

    Write-Step "Ensuring project-local Rust components"
    Invoke-Checked -FilePath $rustupExe -Arguments @(
        "toolchain", "install", "stable-x86_64-pc-windows-msvc",
        "--profile", "minimal",
        "--component", "rustfmt",
        "--component", "clippy"
    )
    Invoke-Checked -FilePath $rustupExe -Arguments @("default", "stable-x86_64-pc-windows-msvc")
    Invoke-Checked -FilePath $rustupExe -Arguments @("target", "add", "x86_64-pc-windows-msvc")

    return $cargoExe
}

if ($env:OS -ne "Windows_NT") {
    throw "tools/build/windows.ps1 must be run on Windows."
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = (Resolve-Path (Join-Path $scriptDir "..\..")).Path
$buildToolsRoot = Join-Path $projectRoot ".build-tools\windows-msvc"
$localToolsRoot = Join-Path $buildToolsRoot "local-tools"
$downloadsDir = Join-Path $localToolsRoot "downloads"
$cargoHome = Join-Path $localToolsRoot "cargo"
$rustupHome = Join-Path $localToolsRoot "rustup"
$buildRoot = Join-Path $buildToolsRoot "build"
$cargoTargetDir = Join-Path $buildRoot "target"
$releaseDir = Join-Path $projectRoot "release\windows"
$artifactDir = Join-Path $cargoTargetDir "x86_64-pc-windows-msvc\release"
$binaryPath = Join-Path $artifactDir "LANScanner.exe"
$releaseBinaryPath = Join-Path $releaseDir "LANScanner.exe"

New-DirectoryIfMissing -Path $buildToolsRoot
New-DirectoryIfMissing -Path $downloadsDir
New-DirectoryIfMissing -Path $cargoHome
New-DirectoryIfMissing -Path $rustupHome
New-DirectoryIfMissing -Path $buildRoot
New-DirectoryIfMissing -Path $releaseDir

if ($Clean) {
    Write-Step "Cleaning project-local Windows build and release directories"
    if (Test-Path -LiteralPath $buildRoot) {
        Remove-Item -LiteralPath $buildRoot -Recurse -Force
    }
    if (Test-Path -LiteralPath $releaseDir) {
        Remove-Item -LiteralPath $releaseDir -Recurse -Force
    }
    New-DirectoryIfMissing -Path $buildRoot
    New-DirectoryIfMissing -Path $releaseDir
}

$env:CARGO_TARGET_DIR = $cargoTargetDir

Ensure-MsvcEnvironment
$cargoExe = Ensure-LocalRustToolchain -CargoHome $cargoHome -RustupHome $rustupHome -DownloadsDir $downloadsDir

Write-Step "Building LANScanner for x86_64-pc-windows-msvc"
Invoke-Checked -FilePath $cargoExe -Arguments @(
    "build",
    "--manifest-path", (Join-Path $projectRoot "Cargo.toml"),
    "--locked",
    "--release",
    "--target", "x86_64-pc-windows-msvc",
    "-p", "lanscanner-app"
)

if (-not (Test-Path -LiteralPath $binaryPath)) {
    throw "Expected Windows artifact not found: $binaryPath"
}

Write-Step "Refreshing release/windows with only LANScanner.exe"
if (Test-Path -LiteralPath $releaseDir) {
    Remove-Item -LiteralPath $releaseDir -Recurse -Force
}
New-DirectoryIfMissing -Path $releaseDir
Copy-Item -LiteralPath $binaryPath -Destination $releaseBinaryPath -Force

Write-Step "Build completed"
Write-Host "Executable: $releaseBinaryPath"
