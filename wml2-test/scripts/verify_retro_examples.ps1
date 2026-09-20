param(
    [Parameter(Mandatory = $true)]
    [string]$SampleRoot,
    [Parameter(Mandatory = $true)]
    [string]$OutputRoot
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$sampleRoot = (Resolve-Path -LiteralPath $SampleRoot).Path
$outputRoot = [System.IO.Path]::GetFullPath($OutputRoot)
$env:CARGO_TARGET_DIR = Join-Path $outputRoot 'cargo-target'

if (-not (Get-Command magick -ErrorAction SilentlyContinue)) {
    throw 'ImageMagick magick.exe is required for the independent comparison gate.'
}

New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
$pcxOutput = Join-Path $outputRoot 'pcx'
$tgaOutput = Join-Path $outputRoot 'tga'
$oracleOutput = Join-Path $outputRoot 'imagemagick'
New-Item -ItemType Directory -Force -Path $pcxOutput, $tgaOutput, $oracleOutput | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $oracleOutput 'pcx'), (Join-Path $oracleOutput 'tga') | Out-Null

function Invoke-CargoExample([string[]]$Arguments) {
    Push-Location $repoRoot
    try {
        & cargo @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "cargo example failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
}

function Assert-CargoExampleFailure([string[]]$Arguments, [string]$Description) {
    Push-Location $repoRoot
    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $output = (& cargo @Arguments 2>&1 | Out-String)
        $exitCode = $LASTEXITCODE
        if ($exitCode -eq 0) {
            throw "$Description unexpectedly succeeded"
        }
        Write-Host "$Description failed as expected"
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
        Pop-Location
    }
}

function Get-FormatFiles([string]$Directory, [string]$Extension) {
    @(Get-ChildItem -LiteralPath $Directory -File | Where-Object { $_.Extension -ieq $Extension } | Sort-Object FullName)
}

function Get-TgaAllZeroAlpha([System.IO.FileInfo]$Source) {
    $data = [System.IO.File]::ReadAllBytes($Source.FullName)
    if ($data.Length -lt 18 -or $data[2] -ne 2 -or $data[16] -ne 32) {
        return $null
    }

    $width = [BitConverter]::ToUInt16($data, 12)
    $height = [BitConverter]::ToUInt16($data, 14)
    $paletteBytes = [BitConverter]::ToUInt16($data, 5) * [int][math]::Ceiling($data[7] / 8)
    $imageOffset = 18 + $data[0] + $paletteBytes
    $imageBytes = [int]$width * [int]$height * 4
    $imageEnd = $imageOffset + $imageBytes
    if ($imageEnd -gt $data.Length) {
        return $null
    }

    for ($offset = $imageOffset + 3; $offset -lt $imageEnd; $offset += 4) {
        if ($data[$offset] -ne 0) {
            return $null
        }
    }

    [pscustomobject]@{ Width = $width; Height = $height }
}

function Assert-ImageMagickMatch([System.IO.FileInfo]$Source, [string]$RustOutputDirectory, [string]$OracleDirectory) {
    $previousErrorActionPreference = $ErrorActionPreference
    $previousNativeCommandUseErrorActionPreference = $PSNativeCommandUseErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $PSNativeCommandUseErrorActionPreference = $false
        $rustOutput = Join-Path $RustOutputDirectory ($Source.Name + '.bmp')
        if (-not (Test-Path -LiteralPath $rustOutput)) {
            throw "missing converter output: $rustOutput"
        }

        $oracleOutput = Join-Path $OracleDirectory ($Source.BaseName + '.bmp')
        $allZeroAlpha = if ($Source.Extension -ieq '.tga') { Get-TgaAllZeroAlpha $Source } else { $null }
        if ($null -ne $allZeroAlpha) {
            # ImageMagick treats an all-zero TGA alpha plane as opaque. Apply
            # WML2's explicit transparent-pixel normalization for this edge case.
            $geometry = '{0}x{1}' -f $allZeroAlpha.Width, $allZeroAlpha.Height
            & magick -size $geometry xc:black "BMP3:$oracleOutput"
        } else {
            & magick $Source.FullName -depth 8 -channel RGB -fx 'a==0 ? 0 : u' +channel -alpha off "BMP3:$oracleOutput"
        }
        if ($LASTEXITCODE -ne 0) {
            throw "ImageMagick normalization failed: $($Source.FullName)"
        }

        $metricPath = Join-Path $OracleDirectory ($Source.BaseName + '.metric.txt')
        $stdoutPath = Join-Path $OracleDirectory ($Source.BaseName + '.stdout.txt')
        $compareProcess = Start-Process -FilePath 'magick.exe' -ArgumentList @(
            'compare', '-metric', 'AE', $rustOutput, $oracleOutput, 'null:'
        ) -Wait -NoNewWindow -RedirectStandardError $metricPath -RedirectStandardOutput $stdoutPath -PassThru
        $compareExitCode = $compareProcess.ExitCode
        $metric = (Get-Content -Raw -LiteralPath $metricPath).Trim()
        if ($compareExitCode -ne 0 -or $metric -notmatch '^0(?:\s|$)') {
            throw "ImageMagick mismatch for $($Source.Name): $metric"
        }
    }
    finally {
        $PSNativeCommandUseErrorActionPreference = $previousNativeCommandUseErrorActionPreference
        $ErrorActionPreference = $previousErrorActionPreference
    }
}

$pcxFiles = Get-FormatFiles (Join-Path $sampleRoot 'pcx') '.pcx'
$pcxValid = @($pcxFiles | Where-Object { $_.Name -cne 'GMARBLES.PCX' })
if ($pcxValid.Count -ne 52) {
    throw "expected 52 valid PCX samples, found $($pcxValid.Count)"
}

$pcxArguments = @('run', '--offline', '-p', 'wml2-test', '--example', 'converter', '--')
$pcxArguments += @($pcxValid.FullName)
$pcxArguments += @('-o', $pcxOutput, '-f', 'bmp')
Invoke-CargoExample $pcxArguments
$pcxOutputs = @(Get-ChildItem -LiteralPath $pcxOutput -Filter '*.bmp' -File)
if ($pcxOutputs.Count -ne $pcxValid.Count) {
    throw "expected $($pcxValid.Count) PCX BMP outputs, found $($pcxOutputs.Count)"
}

$gmarbles = Join-Path $sampleRoot 'pcx\GMARBLES.PCX'
Assert-CargoExampleFailure @(
    'run', '--offline', '-p', 'wml2-test', '--example', 'converter', '--',
    $gmarbles, '-o', (Join-Path $outputRoot 'negative-gmarbles'), '-f', 'bmp'
) 'GMARBLES.PCX converter check'

$tgaFiles = Get-FormatFiles (Join-Path $sampleRoot 'tga') '.tga'
if ($tgaFiles.Count -ne 17) {
    throw "expected 17 TGA samples, found $($tgaFiles.Count)"
}

$tgaArguments = @('run', '--offline', '-p', 'wml2-test', '--example', 'converter', '--')
$tgaArguments += @($tgaFiles.FullName)
$tgaArguments += @('-o', $tgaOutput, '-f', 'bmp')
Invoke-CargoExample $tgaArguments
$tgaOutputs = @(Get-ChildItem -LiteralPath $tgaOutput -Filter '*.bmp' -File)
if ($tgaOutputs.Count -ne $tgaFiles.Count) {
    throw "expected $($tgaFiles.Count) TGA BMP outputs, found $($tgaOutputs.Count)"
}

$unusedCmap = Join-Path $sampleRoot 'tga\b5-unused-cmap.tga'
$unusedCmapLog = Join-Path $oracleOutput 'b5-unused-cmap.magick.log'
$unusedCmapStdout = Join-Path $oracleOutput 'b5-unused-cmap.magick.stdout'
$unusedCmapOutput = Join-Path $oracleOutput 'b5-unused-cmap.bmp'
$unusedCmapProcess = Start-Process -FilePath 'magick.exe' -ArgumentList @(
    $unusedCmap, "BMP3:$unusedCmapOutput"
) -Wait -NoNewWindow -RedirectStandardError $unusedCmapLog -RedirectStandardOutput $unusedCmapStdout -PassThru
if ($unusedCmapProcess.ExitCode -eq 0) {
    throw 'ImageMagick unexpectedly accepted b5-unused-cmap.tga'
}
Write-Host 'b5-unused-cmap.tga: WML2 accepted; ImageMagick rejected as expected'

foreach ($source in $pcxValid) {
    Assert-ImageMagickMatch $source $pcxOutput (Join-Path $oracleOutput 'pcx')
}

$tgaComparable = @($tgaFiles | Where-Object { $_.Name -cne 'b5-unused-cmap.tga' })
if ($tgaComparable.Count -ne 16) {
    throw "expected 16 ImageMagick-comparable TGA samples, found $($tgaComparable.Count)"
}
foreach ($source in $tgaComparable) {
    Assert-ImageMagickMatch $source $tgaOutput (Join-Path $oracleOutput 'tga')
}

$metadataPcx = $pcxValid[0]
$metadataTga = $tgaFiles[0]
foreach ($source in @($metadataPcx, $metadataTga)) {
    Push-Location $repoRoot
    $previousErrorActionPreference = $ErrorActionPreference
    $previousNativeCommandUseErrorActionPreference = $PSNativeCommandUseErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $PSNativeCommandUseErrorActionPreference = $false
        $metadata = (& cargo run --offline -p wml2-test --example metadata -- $source.FullName 2>&1 | Out-String)
        if ($LASTEXITCODE -ne 0) {
            throw "metadata example failed for $($source.Name)"
        }
        if ($metadata -notmatch 'Format: (PCX|TGA)' -or $metadata -notmatch 'width:' -or $metadata -notmatch 'height:') {
            throw "metadata example omitted basic metadata for $($source.Name): $metadata"
        }
    }
    finally {
        $PSNativeCommandUseErrorActionPreference = $previousNativeCommandUseErrorActionPreference
        $ErrorActionPreference = $previousErrorActionPreference
        Pop-Location
    }
}

Write-Host "PCX: $($pcxValid.Count)/$($pcxValid.Count) converted and ImageMagick-matched"
Write-Host "TGA: $($tgaComparable.Count)/$($tgaComparable.Count) ImageMagick-matched; b5-unused-cmap.tga accepted by WML2 by policy"
Write-Host "metadata: PCX and TGA basic metadata verified"
