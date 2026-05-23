# Whisper Integration Test Report

- Backend: `cuda`
- Hardware: `NVIDIA GeForce RTX 3090 (device 0, backend=cuda)`
- Model: `/home/fractal-tess/.local/share/shadowword/models/ggml-medium.bin`
- Clips processed: `4`
- Total audio seconds: `75.00`
- Total inference seconds: `2.42`
- Aggregate realtime factor: `30.95x`
- Average WER: `6.52%`

## Per-clip results

| Clip | Audio (s) | Inference (s) | Realtime (x) | WER (%) |
|---|---:|---:|---:|---:|
| clip_10s | 10.00 | 0.32 | 31.29 | 0.00% |
| clip_15s | 15.00 | 0.41 | 36.33 | 14.29% |
| clip_20s | 20.00 | 0.60 | 33.57 | 3.64% |
| clip_30s | 30.00 | 1.09 | 27.40 | 8.16% |

## Transcripts and diffs

`-word` = expected-only, `+word` = predicted-only

### clip_10s
Reference: All right, everybody, this is just a developing situation, so this video will have some level of incomplete information, but nonetheless, this hack is nuts.
Transcription: All right everybody this is just a developing situation so this video will have some level of incomplete information But nonetheless this hack is nuts
Word diff: all right everybody this is just a developing situation so this video will have some level of incomplete information but nonetheless this hack is nuts

### clip_15s
Reference: the hack and trust me, it is cinema. Absolute cinema. But of course, before we begin, the bag. Here at Terminal, we love PlanetScale. We've been using PlanetScale since day one of Terminal dot
Transcription: the hack and trust me it is cinema absolute cinema but of course before we begin the bag. Here at Terminal we love planet scale we've been using planet scale since day one of terminal.com
Word diff: the hack and trust me it is cinema absolute cinema but of course before we begin the bag here at terminal we love -planetscale +planet +scale we ve been using -planetscale +planet +scale since day one of terminal -dot +com

### clip_20s
Reference: to this disaster and really what is the disaster. So first off, this tweet was sent on May 19th, 2026. We are investigating unauthorized access to GitHub's internal repositories. While we currently have no evidence of impact to customer information stored outside of GitHub's internal repositories, such as our customers' enterprise, organizational repository
Transcription: to this disaster and really what is the disaster so first off this tweet was sent on May 19th 2026 we are investigating unauthorized access to github's internal repositories while we currently have no evidence of impact to customer information stored outside of github's internal repositories such as our customers enterprise organization or repository
Word diff: to this disaster and really what is the disaster so first off this tweet was sent on may 19th 2026 we are investigating unauthorized access to github s internal repositories while we currently have no evidence of impact to customer information stored outside of github s internal repositories such as our customers enterprise -organizational +organization +or repository

### clip_30s
Reference: Believable personally i'm going to be going through and probably rolling a lot of my credentials because shai halud has been getting everybody but this is not shai halud then about 24 hours later not even we see this beautiful little post right here from team pcp please pcp don't please don't hack me i'm just please i'm not worth it just trust me so pcp writes the following hello again breached hope everyone is doing well we're by the way crazy way to start off a message just like Yay, everybody!
Transcription: Personally, I'm gonna be going through and probably rolling a lot of my credentials because Shy Halood has been getting everybody but this is not Shy Halood then about 24 hours later Not even we see this beautiful little post right here from team PCP, please PCP don't please don't hack me I'm just please I'm not worth it. Just trust me. So PCP writes the following. Hello again breached Hope everyone is doing well We're by the way crazy way to start off a message just like everybody
Word diff: -believable personally i m -going -to +gonna be going through and probably rolling a lot of my credentials because -shai -halud +shy +halood has been getting everybody but this is not -shai -halud +shy +halood then about 24 hours later not even we see this beautiful little post right here from team pcp please pcp don t please don t hack me i m just please i m not worth it just trust me so pcp writes the following hello again breached hope everyone is doing well we re by the way crazy way to start off a message just like -yay everybody

