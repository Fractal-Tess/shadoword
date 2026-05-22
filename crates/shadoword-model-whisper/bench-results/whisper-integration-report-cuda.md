# Whisper Integration Test Report

- Backend: `cuda`
- GPU used: `NVIDIA GeForce RTX 3090 (device 0, SHADOWORD_WHISPER_GPU_DEVICE default)`
- Model: `/home/fractal-tess/.local/share/shadowword/models/ggml-large-v3-turbo.bin`
- Clips processed: `4`
- Total audio seconds: `75.00`
- Total inference seconds: `1.86`
- Aggregate realtime factor: `40.22x`
- Average WER: `5.93%`

## Per-clip results

| Clip | Audio (s) | Inference (s) | Realtime (x) | WER (%) |
|---|---:|---:|---:|---:|
| clip_10s | 10.00 | 0.28 | 36.13 | 3.12% |
| clip_15s | 15.00 | 0.34 | 44.01 | 4.88% |
| clip_20s | 20.00 | 0.53 | 37.74 | 13.95% |
| clip_30s | 30.00 | 0.72 | 41.83 | 1.75% |
