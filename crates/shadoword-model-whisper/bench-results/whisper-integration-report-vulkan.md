# Whisper Integration Test Report

- Backend: `vulkan`
- GPU used: `NVIDIA GeForce RTX 3090 (device 0, SHADOWORD_WHISPER_GPU_DEVICE default)`
- Model: `/home/fractal-tess/.local/share/shadowword/models/ggml-large-v3-turbo.bin`
- Clips processed: `4`
- Total audio seconds: `75.00`
- Total inference seconds: `2.93`
- Aggregate realtime factor: `25.59x`
- Average WER: `5.93%`

## Per-clip results

| Clip | Audio (s) | Inference (s) | Realtime (x) | WER (%) |
|---|---:|---:|---:|---:|
| clip_10s | 10.00 | 0.39 | 25.88 | 3.12% |
| clip_15s | 15.00 | 0.37 | 40.69 | 4.88% |
| clip_20s | 20.00 | 0.58 | 34.62 | 13.95% |
| clip_30s | 30.00 | 1.60 | 18.77 | 1.75% |
