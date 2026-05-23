# Whisper Model Size Comparison (VULKAN, GPU 0)

- Corpus: `bench_corpus` (clip_10s/15s/20s/30s)
- Backend: `vulkan`
- GPU device: `0`

## Summary table

| Model | Total inference (s) | Realtime (x) | Avg WER (%) | Report |
|---|---:|---:|---:|---|
| tiny | 2.46 | 30.51 | 12.21 | `whisper-integration-report-vulkan-tiny.md` |
| base | 3.20 | 23.43 | 10.46 | `whisper-integration-report-vulkan-base.md` |
| small | 5.52 | 13.58 | 7.80 | `whisper-integration-report-vulkan-small.md` |
| large-v3-turbo | 5.66 | 13.26 | 3.77 | `whisper-integration-report-vulkan-large-v3-turbo.md` |
| medium | 11.14 | 6.73 | 8.01 | `whisper-integration-report-vulkan-medium.md` |

## Ranking

### Fastest (lower total inference is better)
1. `tiny` - 2.46s (30.51x)
2. `base` - 3.20s (23.43x)
3. `small` - 5.52s (13.58x)
4. `large-v3-turbo` - 5.66s (13.26x)
5. `medium` - 11.14s (6.73x)

### Best accuracy (lower WER is better)
1. `large-v3-turbo` - 3.77% WER
2. `small` - 7.80% WER
3. `medium` - 8.01% WER
4. `base` - 10.46% WER
5. `tiny` - 12.21% WER
