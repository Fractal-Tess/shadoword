# Whisper Integration Test Report

- Backend: `vulkan`
- Hardware: `NVIDIA GeForce RTX 3090 (device 0, backend=vulkan)`
- Model: `/home/fractal-tess/.local/share/shadowword/models/ggml-medium.bin`
- Clips processed: `4`
- Total audio seconds: `75.00`
- Total inference seconds: `11.14`
- Aggregate realtime factor: `6.73x`
- Average WER: `8.01%`

## Per-clip results

| Clip | Audio (s) | Inference (s) | Realtime (x) | WER (%) |
|---|---:|---:|---:|---:|
| clip_10s | 10.00 | 1.52 | 6.60 | 8.00% |
| clip_15s | 15.00 | 2.00 | 7.49 | 14.29% |
| clip_20s | 20.00 | 2.97 | 6.74 | 3.64% |
| clip_30s | 30.00 | 4.65 | 6.45 | 6.12% |

## Transcripts and diffs

`-word` = expected-only, `+word` = predicted-only

### clip_10s
Reference: All right, everybody, this is just a developing situation, so this video will have some level of incomplete information, but nonetheless, this hack is nuts.
Transcription: All right, everybody. This is just a developing situation. So this video will have some level of incomplete information But nonetheless this hack is nuts. What's
Word diff: all right everybody this is just a developing situation so this video will have some level of incomplete information but nonetheless this hack is nuts +what +s

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
Transcription: Personally, I'm going to be going through and probably rolling a lot of my credentials because Shy Halood has been getting everybody but this is not Shy Halood. Then about 24 hours later not even we see this beautiful little post right here from team PCP. Please PCP. Don't please don't hack me. I'm just please. I'm not worth it Just trust me. So PCP writes the following. Hello again breached. Hope everyone is doing well We're by the way crazy way to start off a message. Just like hey everybody
Word diff: -believable personally i m going to be going through and probably rolling a lot of my credentials because -shai -halud +shy +halood has been getting everybody but this is not -shai -halud +shy +halood then about 24 hours later not even we see this beautiful little post right here from team pcp please pcp don t please don t hack me i m just please i m not worth it just trust me so pcp writes the following hello again breached hope everyone is doing well we re by the way crazy way to start off a message just like -yay +hey everybody

