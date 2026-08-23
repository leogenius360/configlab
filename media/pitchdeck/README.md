# ConfigLab pitch-deck sources

This package is a product and engineering pitch aimed at engineering leaders,
platform teams, early Rust adopters, and ecosystem sponsors.

## Sources

- `SCRIPT.md` — narration transcript;
- `assets/` — three original AI-generated visuals used by the deck.

Generated slides, presentations, video, narration, validation output, and the
distribution ZIP are intentionally excluded from Git. Published artifacts
belong on the matching GitHub Release.

The claims are intentionally limited to behavior evidenced by this repository.
The deck identifies the remote control plane, secret management, distributed
rollout, and external side-effect coordination as integration boundaries rather
than implemented product capabilities.

## Rebuild

Create a Python environment with `pillow`, `python-pptx`, `pypdf`, and
`imageio-ffmpeg`, then run:

```powershell
python .\script\pitchdeck\build_deck.py
python .\script\pitchdeck\validate_deck.py
```

Narration is generated locally with the Windows `Microsoft Zira Desktop`
speech voice. The generated source visuals were created with the built-in image
generation tool; their final prompt specifications are recorded in
`IMAGE-PROMPTS.md`.
