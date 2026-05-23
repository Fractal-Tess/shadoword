# Whisper Integration Test Report

- Backend: `vulkan`
- Hardware: `NVIDIA GeForce RTX 3090 (device 0, backend=vulkan)`
- Model: `/home/fractal-tess/.local/share/shadowword/models/ggml-large-v3-turbo.bin`
- Clips processed: `4`
- Total audio seconds: `75.00`
- Total inference seconds: `5.66`
- Aggregate realtime factor: `13.26x`
- Average WER: `3.77%`

## Per-clip results

| Clip | Audio (s) | Inference (s) | Realtime (x) | WER (%) |
|---|---:|---:|---:|---:|
| clip_10s | 10.00 | 1.01 | 9.93 | 0.00% |
| clip_15s | 15.00 | 1.07 | 13.97 | 11.43% |
| clip_20s | 20.00 | 1.45 | 13.77 | 3.64% |
| clip_30s | 30.00 | 2.12 | 14.12 | 0.00% |

## Transcripts and diffs

`-word` = expected-only, `+word` = predicted-only

### clip_10s
Reference: All right, everybody, this is just a developing situation, so this video will have some level of incomplete information, but nonetheless, this hack is nuts.
Transcription: All right, everybody, this is just a developing situation, so this video will have some level of incomplete information, but nonetheless, this hack is nuts.
Word diff: all right everybody this is just a developing situation so this video will have some level of incomplete information but nonetheless this hack is nuts

### clip_15s
Reference: the hack and trust me, it is cinema. Absolute cinema. But of course, before we begin, the bag. Here at Terminal, we love PlanetScale. We've been using PlanetScale since day one of Terminal dot
Transcription: the hack and trust me it is cinema absolute cinema but of course before we begin the bag here at terminal we love planet scale we've been using planet scale since day one of terminal dot
Word diff: the hack and trust me it is cinema absolute cinema but of course before we begin the bag here at terminal we love -planetscale +planet +scale we ve been using -planetscale +planet +scale since day one of terminal dot

### clip_20s
Reference: to this disaster and really what is the disaster. So first off, this tweet was sent on May 19th, 2026. We are investigating unauthorized access to GitHub's internal repositories. While we currently have no evidence of impact to customer information stored outside of GitHub's internal repositories, such as our customers' enterprise, organizational repository
Transcription: to this disaster, and really, what is the disaster? So first off, this tweet was sent on May 19th, 2026. We are investigating unauthorized access to GitHub's internal repositories. While we currently have no evidence of impact to customer information stored outside of GitHub's internal repositories, such as our customer's enterprise, organizational repository,
Word diff: to this disaster and really what is the disaster so first off this tweet was sent on may 19th 2026 we are investigating unauthorized access to github s internal repositories while we currently have no evidence of impact to customer information stored outside of github s internal repositories such as our -customers +customer +s enterprise organizational repository

### clip_30s
Reference: Believable personally i'm going to be going through and probably rolling a lot of my credentials because shai halud has been getting everybody but this is not shai halud then about 24 hours later not even we see this beautiful little post right here from team pcp please pcp don't please don't hack me i'm just please i'm not worth it just trust me so pcp writes the following hello again breached hope everyone is doing well we're by the way crazy way to start off a message just like Yay, everybody!
Transcription: believable personally i'm going to be going through and probably rolling a lot of my credentials because shai halud has been getting everybody but this is not shai halud then about 24 hours later not even we see this beautiful little post right here from team pcp please pcp don't please don't hack me i'm just please i'm not worth it just trust me so pcp writes the following hello again breached hope everyone is doing well we're by the way crazy way to start off a message just like Yay, everybody!
Word diff: believable personally i m going to be going through and probably rolling a lot of my credentials because shai halud has been getting everybody but this is not shai halud then about 24 hours later not even we see this beautiful little post right here from team pcp please pcp don t please don t hack me i m just please i m not worth it just trust me so pcp writes the following hello again breached hope everyone is doing well we re by the way crazy way to start off a message just like yay everybody

