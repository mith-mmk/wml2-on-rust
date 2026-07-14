[CmdletBinding()]
param(
    [switch]$DownloadMissing,
    [switch]$KeepWork
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$externalRoot = Join-Path $repoRoot 'test/images/external'
$workRoot = Join-Path $repoRoot '.test-avif-script'
$downloadRoot = Join-Path $workRoot 'download'
$outputRoot = Join-Path $workRoot 'outputs'
$sourceCommit = 'c666a368b73006246694919b5dbcc078317af6cc'
$sourceBase = "https://raw.githubusercontent.com/link-u/avif-sample-images/$sourceCommit"

$manifest = @(
    [pscustomobject]@{ Name = 'fox.profile1.8bpc.yuv444.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile1.8bpc.yuv444.avif'; Sha256 = 'a0cdc981a6b056c8af2d177a1438c332d630040dacbfd1c89bb5e3e381ba5822'; Width = 1204; Height = 800 }
    [pscustomobject]@{ Name = 'fox.profile1.8bpc.yuv444.odd-height.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile1.8bpc.yuv444.odd-height.avif'; Sha256 = '77e0510def73213c00ebcf051cf45fa63cf27d7c69340cc145ab6d44ec77bb07'; Width = 1204; Height = 799 }
    [pscustomobject]@{ Name = 'fox.profile1.8bpc.yuv444.odd-width.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile1.8bpc.yuv444.odd-width.avif'; Sha256 = '12787042364bd13be01830f988cb714220bde340a3329baa808df27a269b83f8'; Width = 1203; Height = 800 }
    [pscustomobject]@{ Name = 'fox.profile1.8bpc.yuv444.odd-width.odd-height.avif'; Kind = 'supported'; RelativePath = 'avif/supported/fox.profile1.8bpc.yuv444.odd-width.odd-height.avif'; Sha256 = 'fad3b6dd9cb99e6925858f69aafae3f68c861845f2c3d4a6d1c51c6161490134'; Width = 1203; Height = 799 }
    [pscustomobject]@{ Name = 'fox.profile0.8bpc.yuv420.avif'; Kind = 'unsupported'; RelativePath = 'avif/unsupported/fox.profile0.8bpc.yuv420.avif'; Sha256 = 'cb884c82ac7b6d4fa03b1f687e9e20abc346107095473e9c1d422aaf0de14eaf'; ErrorPattern = '4:4:4 color only' }
    [pscustomobject]@{ Name = 'fox.profile0.8bpc.yuv420.monochrome.avif'; Kind = 'unsupported'; RelativePath = 'avif/unsupported/fox.profile0.8bpc.yuv420.monochrome.avif'; Sha256 = '15d84077066c47fdbe2a7b8ed583a17017d09a033144ac1b31486d6c8f6f5c82'; ErrorPattern = '4:4:4 color only' }
    [pscustomobject]@{ Name = 'fox.profile1.10bpc.yuv444.avif'; Kind = 'unsupported'; RelativePath = 'avif/unsupported/fox.profile1.10bpc.yuv444.avif'; Sha256 = 'a10de8204aee73ba1786daca6390546bd7aa6b069aaa644012219a1c11246a43'; ErrorPattern = '10-bit quantization is not supported' }
    [pscustomobject]@{ Name = 'fox.profile2.8bpc.yuv422.avif'; Kind = 'unsupported'; RelativePath = 'avif/unsupported/fox.profile2.8bpc.yuv422.avif'; Sha256 = '2cb363d30f83bff58ee049874b1808b37cb1d35342edf16b3ce25cb243c9ea55'; ErrorPattern = '4:4:4 color only' }
    [pscustomobject]@{ Name = 'plum-blossom-small.profile1.8bpc.yuv444.alpha-full.avif'; Kind = 'unsupported'; RelativePath = 'avif/unsupported/plum-blossom-small.profile1.8bpc.yuv444.alpha-full.avif'; Sha256 = '245a3dad6371dc702f29eb7e9735f843b63c525da871859728bedbe5bb274985'; ErrorPattern = 'AVIF alpha auxiliary item composition is not supported yet' }
    [pscustomobject]@{ Name = 'kimono.crop.avif'; Kind = 'unsupported'; RelativePath = 'avif/unsupported/kimono.crop.avif'; Sha256 = 'f175dcd9c64813b759da185fa67076fb772b76059845b2aad3ddcfab257f75ad'; ErrorPattern = 'AVIF clap/irot/imir composition is not supported yet' }
    [pscustomobject]@{ Name = 'kimono.mirror-horizontal.avif'; Kind = 'unsupported'; RelativePath = 'avif/unsupported/kimono.mirror-horizontal.avif'; Sha256 = '2bbc004d91145488610158a5acdb4d706495a2b15511db20ff57bb9efd80885c'; ErrorPattern = 'AVIF clap/irot/imir composition is not supported yet' }
    [pscustomobject]@{ Name = 'kimono.rotate90.avif'; Kind = 'unsupported'; RelativePath = 'avif/unsupported/kimono.rotate90.avif'; Sha256 = 'bd1157d8c840713c82b907b9d3ae80bc3817849e11c323d875f8016e035bd3cc'; ErrorPattern = 'AVIF clap/irot/imir composition is not supported yet' }
    [pscustomobject]@{ Name = 'red-at-12-oclock-with-color-profile-8bpc.avif'; Kind = 'unsupported'; RelativePath = 'avif/unsupported/red-at-12-oclock-with-color-profile-8bpc.avif'; Sha256 = '3e6f2f4016e66e3c94707eaa8373e6f582321e005964cd35b64bc183e1bf10ea'; ErrorPattern = 'AVIF ICC colour management for RGBA conversion is not supported yet' }
    [pscustomobject]@{ Name = 'star-8bpc.avifs'; Kind = 'unsupported'; RelativePath = 'avif/unsupported/star-8bpc.avifs'; Sha256 = 'ae35b161de67a5afeb195ee401f369c34990f0ff8662f70ab4065bc6931f0a66'; ErrorPattern = 'AVIF sequences are not supported' }
)

function Read-BigEndianUInt32([byte[]]$bytes, [int]$offset) {
    return ([uint32]$bytes[$offset] -shl 24) -bor
        ([uint32]$bytes[$offset + 1] -shl 16) -bor
        ([uint32]$bytes[$offset + 2] -shl 8) -bor
        [uint32]$bytes[$offset + 3]
}

function Test-PngDimensions([string]$path, [int]$width, [int]$height) {
    $bytes = [IO.File]::ReadAllBytes($path)
    if ($bytes.Length -lt 24) { return $false }
    $signature = @(137, 80, 78, 71, 13, 10, 26, 10)
    for ($index = 0; $index -lt $signature.Count; $index++) {
        if ($bytes[$index] -ne $signature[$index]) { return $false }
    }
    $chunkLength = Read-BigEndianUInt32 $bytes 8
    $chunkType = [Text.Encoding]::ASCII.GetString($bytes, 12, 4)
    return $chunkLength -eq 13 -and $chunkType -eq 'IHDR' -and
        (Read-BigEndianUInt32 $bytes 16) -eq $width -and
        (Read-BigEndianUInt32 $bytes 20) -eq $height
}

$oldLocation = Get-Location
$oldTemp = $env:TEMP
$oldTmp = $env:TMP
$failures = [System.Collections.Generic.List[string]]::new()
$successCount = 0
$expectedFailureCount = 0
$unexpectedCount = 0
$partialPngCount = 0

try {
    Set-Location $repoRoot
    New-Item -ItemType Directory -Force -Path $downloadRoot, $outputRoot | Out-Null
    $env:TEMP = Join-Path $workRoot 'temp'
    $env:TMP = $env:TEMP
    New-Item -ItemType Directory -Force -Path $env:TEMP | Out-Null

    foreach ($entry in $manifest) {
        $target = Join-Path $externalRoot $entry.RelativePath
        try {
            if (-not (Test-Path -LiteralPath $target -PathType Leaf)) {
                if (-not $DownloadMissing) {
                    throw "missing sample; rerun with -DownloadMissing: $target"
                }

                $download = Join-Path $downloadRoot $entry.Name
                Invoke-WebRequest -UseBasicParsing -Uri "$sourceBase/$($entry.Name)" -OutFile $download
                New-Item -ItemType Directory -Force -Path (Split-Path $target) | Out-Null
                Move-Item -LiteralPath $download -Destination $target
            }

            $bytes = [System.IO.File]::ReadAllBytes($target)
            if ($bytes.Length -lt 12) {
                throw 'file is shorter than an AVIF ftyp header'
            }
            $brand = [Text.Encoding]::ASCII.GetString($bytes, 4, 8)
            if ($brand -notmatch '^ftypavi[fs]$') {
                throw "unexpected AVIF brand: $brand"
            }

            $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $target).Hash.ToLowerInvariant()
            if ($hash -ne $entry.Sha256) {
                throw "SHA-256 mismatch: expected $($entry.Sha256), got $hash"
            }

            $probeRoot = Join-Path $outputRoot $entry.Name
            New-Item -ItemType Directory -Force -Path $probeRoot | Out-Null
            $converterOutput = @(& cargo run -q -p wml2-test --example converter -- $target -o $probeRoot -f png 2>&1)
            $converterExit = $LASTEXITCODE
            $converterText = $converterOutput -join "`n"
            $partialOutputs = @(Get-ChildItem -File -Recurse $probeRoot)

            if ($entry.Kind -eq 'supported') {
                $pngOutputs = @($partialOutputs | Where-Object { $_.Extension -ieq '.png' })
                if ($converterExit -ne 0 -or $pngOutputs.Count -ne 1 -or
                    -not (Test-PngDimensions $pngOutputs[0].FullName $entry.Width $entry.Height)) {
                    $unexpectedCount++
                    $failures.Add("$($entry.Name): expected success, exit=$converterExit, pngs=$($pngOutputs.Count), output=$converterText")
                    Write-Host "[FAIL] $($entry.Name) expected success"
                } else {
                    $finalOutput = Join-Path $externalRoot "converted/avif/$($entry.Name).png"
                    New-Item -ItemType Directory -Force -Path (Split-Path $finalOutput) | Out-Null
                    Copy-Item -LiteralPath $pngOutputs[0].FullName -Destination $finalOutput -Force
                    $successCount++
                    Write-Host "[PASS] $($entry.Name) converted $($entry.Width)x$($entry.Height)"
                }
            } else {
                if ($converterExit -eq 0 -or $converterText -notmatch $entry.ErrorPattern -or $partialOutputs.Count -ne 0) {
                    $unexpectedCount++
                    if ($partialOutputs.Count -ne 0) { $partialPngCount += $partialOutputs.Count }
                    $failures.Add("$($entry.Name): exit=$converterExit, expected error '$($entry.ErrorPattern)', partial_outputs=$($partialOutputs.Count), output=$converterText")
                    Write-Host "[FAIL] $($entry.Name) expected failure"
                } else {
                    $expectedFailureCount++
                    Write-Host "[PASS] $($entry.Name) expected failure"
                }
            }
        } catch {
            $failures.Add("$($entry.Name): $($_.Exception.Message)")
            Write-Host "[FAIL] $($entry.Name)"
        }
    }
} finally {
    Set-Location $oldLocation
    $env:TEMP = $oldTemp
    $env:TMP = $oldTmp
    if (-not $KeepWork -and (Test-Path -LiteralPath $workRoot)) {
        Remove-Item -LiteralPath $workRoot -Recurse -Force
    }
}

if ($failures.Count -gt 0) {
    Write-Error ("AVIF compatibility check failed ({0}):`n{1}" -f $failures.Count, ($failures -join "`n"))
    exit 1
}

Write-Host "AVIF compatibility check passed: $successCount successes, $expectedFailureCount expected failures, unexpected $unexpectedCount, partial PNGs $partialPngCount."
exit 0
