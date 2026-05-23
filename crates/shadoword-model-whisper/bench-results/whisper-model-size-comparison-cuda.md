# Whisper Model Size Comparison (CUDA, GPU 0)

- Corpus: `bench_corpus` (clip_10s/15s/20s/30s)
- Backend: `cuda`
- GPU device: `0`

## Summary table

| Model | Total inference (s) | Realtime (x) | Avg WER (%) | Report |
|---|---:|---:|---:|---|
| tiny | 1.10 | 68.05 | 9.88 | `whisper-integration-report-cuda-tiny.md` |
| base | 1.19 | 63.21 | 10.97 | `whisper-integration-report-cuda-base.md` |
| large-v3-turbo | 1.52 | 49.34 | 2.86 | `whisper-integration-report-cuda-large-v3-turbo.md` |
| small | 1.55 | 48.37 | 9.49 | `whisper-integration-report-cuda-small.md` |
| medium | 2.42 | 30.95 | 6.52 | `whisper-integration-report-cuda-medium.md` |

## Ranking

### Fastest (lower total inference is better)
1. `tiny` - 1.10s (68.05x)
2. `base` - 1.19s (63.21x)
3. `large-v3-turbo` - 1.52s (49.34x)
4. `small` - 1.55s (48.37x)
5. `medium` - 2.42s (30.95x)

### Best accuracy (lower WER is better)
1. `large-v3-turbo` - 2.86% WER
2. `medium` - 6.52% WER
3. `small` - 9.49% WER
4. `tiny` - 9.88% WER
5. `base` - 10.97% WER
