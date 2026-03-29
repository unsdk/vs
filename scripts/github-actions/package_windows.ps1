if ($env:BUILD_TARGET -ne "") {
    $binaryPath = "target\$env:BUILD_TARGET\$env:BUILD_PROFILE\vs.exe"
} else {
    $binaryPath = "target\$env:BUILD_PROFILE\vs.exe"
}

$archiveName = "vs-v$env:VERSION-$env:PLATFORM_LABEL-$env:ARTIFACT_VARIANT.zip"
$stagingDir = Join-Path $env:RUNNER_TEMP $env:STAGING_DIR_NAME
New-Item -ItemType Directory -Force -Path $stagingDir | Out-Null
Copy-Item $binaryPath (Join-Path $stagingDir "vs.exe")
Compress-Archive -Path (Join-Path $stagingDir "*") -DestinationPath $archiveName -Force
Add-Content -Path $env:GITHUB_ENV -Value "ARCHIVE_PATH=$archiveName"
