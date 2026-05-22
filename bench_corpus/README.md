# bench_corpus

Benchmark audio corpus for `crates/shadoword-model-whisper/tests/whisper_integration.rs`.

Expected files:

- `clip_10s.wav` / `clip_10s_ref.txt`
- `clip_15s.wav` / `clip_15s_ref.txt`
- `clip_20s.wav` / `clip_20s_ref.txt`
- `clip_30s.wav` / `clip_30s_ref.txt`

The integration test now prefers `bench_corpus/` and falls back to `test_corpus/` if present.
