param(
    [Parameter(Mandatory = $true)][string]$Manifest,
    [Parameter(Mandatory = $true)][string]$Output
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
New-Item -ItemType Directory -Force -Path $Output | Out-Null
$deck = Get-Content -Raw -LiteralPath $Manifest | ConvertFrom-Json
$speaker = New-Object System.Speech.Synthesis.SpeechSynthesizer
$speaker.SelectVoice('Microsoft Zira Desktop')
$speaker.Rate = 0
$speaker.Volume = 100
try {
    foreach ($slide in $deck.slides) {
        $path = Join-Path $Output ('{0:D2}.wav' -f [int]$slide.number)
        $speaker.SetOutputToWaveFile($path)
        $speaker.Speak([string]$slide.narration)
        $speaker.SetOutputToNull()
    }
}
finally {
    $speaker.Dispose()
}
