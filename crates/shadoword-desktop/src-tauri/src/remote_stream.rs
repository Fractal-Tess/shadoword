use anyhow::{anyhow, Context, Result};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use opus::{Application, Channels, Encoder};
use reqwest::Url;
use serde::Deserialize;
use shadoword_core::{AudioInput, StreamingPcmFormat};
use std::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

const OPUS_SAMPLE_RATE: u32 = 48_000;
const OPUS_FRAME_SAMPLES: usize = 960;
const MAX_PACKET_BYTES: usize = 4_000;
const MAX_PCM_PACKET_BYTES: usize = 64 * 1024;
const V2_HANDSHAKE_TIMEOUT: Duration = Duration::from_millis(750);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(120);
const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);

type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
type SocketSender = SplitSink<Socket, Message>;
type SocketReceiver = SplitStream<Socket>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteProtocol {
    V1,
    V2,
    V3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePartial {
    pub segment_index: u64,
    pub text: String,
    pub elapsed_ms: u64,
    pub engine: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDone {
    pub text: String,
    pub segments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteEvent {
    Accepted {
        segment_index: u64,
        outstanding: usize,
        remaining_credit: usize,
        audio_duration_ms: Option<u64>,
        sample_rate: Option<u32>,
    },
    Partial(RemotePartial),
    Done(RemoteDone),
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(alias = "started")]
    Started {
        protocol_version: u8,
        flow_id: String,
        credit: usize,
    },
    #[serde(alias = "accepted")]
    Accepted {
        segment_index: u64,
        outstanding: usize,
        remaining_credit: usize,
        #[serde(default)]
        audio_duration_ms: Option<u64>,
        #[serde(default)]
        sample_rate: Option<u32>,
    },
    #[serde(alias = "partial")]
    Partial {
        segment_index: u64,
        text: String,
        elapsed_ms: u64,
        engine: String,
    },
    #[serde(alias = "done")]
    Done { text: String, segments: Vec<String> },
    #[serde(alias = "error")]
    Error { code: String, message: String },
}

enum Handshake {
    Negotiated {
        protocol: RemoteProtocol,
        flow_id: String,
        credit: usize,
    },
    RetryLegacy,
}

pub struct RemoteStream {
    sender: SocketSender,
    receiver: SocketReceiver,
    encoder: Option<Encoder>,
    protocol: RemoteProtocol,
    flow_id: Option<String>,
    credit: usize,
    sample_rate: u32,
    pcm_format: StreamingPcmFormat,
}

impl RemoteStream {
    #[cfg(test)]
    pub async fn connect(endpoint: &str, token: Option<&str>, sample_rate: u32) -> Result<Self> {
        Self::connect_with_pcm_format(endpoint, token, sample_rate, StreamingPcmFormat::F32le).await
    }

    pub async fn connect_with_pcm_format(
        endpoint: &str,
        token: Option<&str>,
        sample_rate: u32,
        pcm_format: StreamingPcmFormat,
    ) -> Result<Self> {
        let url = stream_endpoint(endpoint)?;
        let mut socket = connect_socket(&url, token).await?;
        send_start(&mut socket, Some(3), sample_rate, pcm_format).await?;
        let first_handshake = negotiate(&mut socket, RemoteProtocol::V3).await?;
        let (socket, protocol, flow_id, credit) = match first_handshake {
            Handshake::Negotiated {
                protocol,
                flow_id,
                credit,
            } => (socket, protocol, Some(flow_id), credit),
            Handshake::RetryLegacy => {
                let _ = tokio::time::timeout(CLOSE_TIMEOUT, socket.close(None)).await;
                let mut legacy = connect_socket(&url, token).await?;
                send_start(&mut legacy, Some(2), OPUS_SAMPLE_RATE, pcm_format).await?;
                match negotiate(&mut legacy, RemoteProtocol::V2).await? {
                    Handshake::Negotiated {
                        protocol,
                        flow_id,
                        credit,
                    } => (legacy, protocol, Some(flow_id), credit),
                    Handshake::RetryLegacy => {
                        let _ = tokio::time::timeout(CLOSE_TIMEOUT, legacy.close(None)).await;
                        let mut fallback = connect_socket(&url, token).await?;
                        send_start(&mut fallback, None, OPUS_SAMPLE_RATE, pcm_format).await?;
                        (fallback, RemoteProtocol::V1, None, 1)
                    }
                }
            }
        };
        let (sender, receiver) = socket.split();
        let encoder = if protocol == RemoteProtocol::V3 {
            None
        } else {
            Some(
                Encoder::new(OPUS_SAMPLE_RATE, Channels::Mono, Application::Voip)
                    .context("failed to initialize Opus encoder")?,
            )
        };
        Ok(Self {
            sender,
            receiver,
            encoder,
            protocol,
            flow_id,
            credit,
            sample_rate: if protocol == RemoteProtocol::V3 {
                sample_rate
            } else {
                OPUS_SAMPLE_RATE
            },
            pcm_format,
        })
    }

    pub fn protocol(&self) -> RemoteProtocol {
        self.protocol
    }

    pub fn flow_id(&self) -> Option<&str> {
        self.flow_id.as_deref()
    }

    pub fn credit(&self) -> usize {
        self.credit
    }

    pub async fn send_samples(&mut self, samples: &[f32]) -> Result<()> {
        if self.protocol != RemoteProtocol::V3 {
            return Err(anyhow!("raw PCM requires remote stream protocol v3"));
        }
        let bytes_per_sample = match self.pcm_format {
            StreamingPcmFormat::S16le => std::mem::size_of::<i16>(),
            StreamingPcmFormat::F32le => std::mem::size_of::<f32>(),
        };
        let byte_limited_samples = MAX_PCM_PACKET_BYTES / bytes_per_sample;
        let duration_limited_samples = (self.sample_rate as usize / 4).max(1);
        let samples_per_packet = byte_limited_samples.min(duration_limited_samples);
        for chunk in samples.chunks(samples_per_packet) {
            let packet = encode_pcm_packet(chunk, self.pcm_format)?;
            tokio::time::timeout(WRITE_TIMEOUT, self.sender.send(Message::binary(packet)))
                .await
                .map_err(|_| anyhow!("timed out sending raw PCM to the remote stream"))?
                .context("failed to send raw PCM to the remote stream")?;
        }
        Ok(())
    }

    pub async fn send_segment(&mut self, segment_index: u64, audio: AudioInput) -> Result<()> {
        if self.protocol == RemoteProtocol::V3 {
            return Err(anyhow!(
                "protocol v3 uses server-side VAD instead of client segment commits"
            ));
        }
        if audio.samples.is_empty() {
            return Err(anyhow!("cannot commit an empty streaming segment"));
        }
        let samples = resample_linear(&audio.samples, audio.sample_rate, OPUS_SAMPLE_RATE)?;
        let encoder = self
            .encoder
            .as_mut()
            .ok_or_else(|| anyhow!("legacy Opus encoder is unavailable"))?;
        for frame in opus_frames(&samples) {
            let packet = encoder
                .encode_vec_float(&frame, MAX_PACKET_BYTES)
                .context("failed to encode microphone audio as Opus")?;
            tokio::time::timeout(WRITE_TIMEOUT, self.sender.send(Message::binary(packet)))
                .await
                .map_err(|_| anyhow!("timed out sending Opus audio to the remote stream"))?
                .context("failed to send Opus packet")?;
        }
        tokio::time::timeout(
            WRITE_TIMEOUT,
            self.sender
                .send(Message::text(commit_message(self.protocol, segment_index))),
        )
        .await
        .map_err(|_| anyhow!("timed out committing a remote stream segment"))?
        .context("failed to commit remote stream segment")
    }

    pub async fn keep_alive(&mut self) -> Result<()> {
        tokio::time::timeout(
            WRITE_TIMEOUT,
            self.sender.send(Message::Ping(Vec::new().into())),
        )
        .await
        .map_err(|_| anyhow!("timed out sending a remote stream keepalive"))?
        .context("failed to keep the remote stream alive")
    }

    pub async fn finish_request(&mut self) -> Result<()> {
        tokio::time::timeout(WRITE_TIMEOUT, self.sender.send(Message::text("Finish")))
            .await
            .map_err(|_| anyhow!("timed out sending the remote stream finish request"))?
            .context("failed to finish remote stream")
    }

    pub async fn next_event(&mut self) -> Result<RemoteEvent> {
        let deadline = tokio::time::Instant::now() + READ_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            let message = tokio::time::timeout(remaining, self.receiver.next())
                .await
                .map_err(|_| {
                    anyhow!(
                        "timed out after {} seconds waiting for a remote streaming response",
                        READ_TIMEOUT.as_secs()
                    )
                })?
                .ok_or_else(|| anyhow!("remote stream closed before sending a response"))?
                .context("remote stream receive failed")?;
            match message {
                Message::Text(text) => match parse_server_message(text.as_str())? {
                    ServerMessage::Accepted {
                        segment_index,
                        outstanding,
                        remaining_credit,
                        audio_duration_ms,
                        sample_rate,
                    } => {
                        return Ok(RemoteEvent::Accepted {
                            segment_index,
                            outstanding,
                            remaining_credit,
                            audio_duration_ms,
                            sample_rate,
                        });
                    }
                    ServerMessage::Partial {
                        segment_index,
                        text,
                        elapsed_ms,
                        engine,
                    } => {
                        return Ok(RemoteEvent::Partial(RemotePartial {
                            segment_index,
                            text,
                            elapsed_ms,
                            engine,
                        }));
                    }
                    ServerMessage::Done { text, segments } => {
                        return Ok(RemoteEvent::Done(RemoteDone { text, segments }));
                    }
                    ServerMessage::Error { code, message } => {
                        return Err(anyhow!("remote stream error ({code}): {message}"));
                    }
                    ServerMessage::Started { .. } => {
                        return Err(anyhow!("remote stream sent a duplicate Started message"));
                    }
                },
                Message::Ping(payload) => {
                    tokio::time::timeout(WRITE_TIMEOUT, self.sender.send(Message::Pong(payload)))
                        .await
                        .map_err(|_| anyhow!("timed out answering a remote stream ping"))?
                        .context("failed to answer remote stream ping")?;
                }
                Message::Close(frame) => {
                    return Err(anyhow!("remote stream closed unexpectedly: {frame:?}"));
                }
                Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
            }
        }
    }

    pub async fn close(mut self) {
        let _ = tokio::time::timeout(CLOSE_TIMEOUT, self.sender.close()).await;
    }
}

async fn connect_socket(url: &Url, token: Option<&str>) -> Result<Socket> {
    let mut request = url
        .as_str()
        .into_client_request()
        .context("failed to create streaming WebSocket request")?;
    if let Some(token) = token.map(str::trim).filter(|token| !token.is_empty()) {
        let mut value = HeaderValue::from_str(&format!("Bearer {token}"))
            .context("bearer token contains invalid HTTP header characters")?;
        value.set_sensitive(true);
        request.headers_mut().insert(AUTHORIZATION, value);
    }
    tokio::time::timeout(CONNECT_TIMEOUT, connect_async(request))
        .await
        .map_err(|_| {
            anyhow!(
                "timed out after {} seconds connecting to the remote streaming endpoint",
                CONNECT_TIMEOUT.as_secs()
            )
        })?
        .map(|(socket, _)| socket)
        .context("failed to connect to remote streaming endpoint")
}

async fn send_start(
    socket: &mut Socket,
    protocol_version: Option<u8>,
    sample_rate: u32,
    pcm_format: StreamingPcmFormat,
) -> Result<()> {
    let message = match protocol_version {
        Some(3) => serde_json::json!({
            "type": "Start",
            "sample_rate": sample_rate,
            "channels": 1,
            "protocol_version": 3,
            "audio_format": match pcm_format {
                StreamingPcmFormat::S16le => "pcm_s16le",
                StreamingPcmFormat::F32le => "pcm_f32le",
            },
        }),
        Some(protocol_version) => serde_json::json!({
            "type": "Start",
            "sample_rate": sample_rate,
            "channels": 1,
            "protocol_version": protocol_version,
        }),
        None => serde_json::json!({
            "type": "Start",
            "sample_rate": sample_rate,
            "channels": 1,
        }),
    };
    tokio::time::timeout(
        WRITE_TIMEOUT,
        socket.send(Message::text(message.to_string())),
    )
    .await
    .map_err(|_| anyhow!("timed out starting the remote stream"))?
    .context("failed to start remote stream")
}

async fn negotiate(socket: &mut Socket, expected: RemoteProtocol) -> Result<Handshake> {
    let deadline = tokio::time::Instant::now() + V2_HANDSHAKE_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let first = tokio::time::timeout(remaining, socket.next()).await;
        match first {
            Err(_) => return Ok(Handshake::RetryLegacy),
            Ok(None | Some(Err(_))) => return Ok(Handshake::RetryLegacy),
            Ok(Some(Ok(Message::Close(_)))) => return Ok(Handshake::RetryLegacy),
            Ok(Some(Ok(Message::Text(text)))) => return handshake_from_message(&text, expected),
            Ok(Some(Ok(Message::Ping(payload)))) => {
                tokio::time::timeout(WRITE_TIMEOUT, socket.send(Message::Pong(payload)))
                    .await
                    .map_err(|_| anyhow!("timed out answering a ping during stream handshake"))?
                    .context("failed to answer remote stream ping during handshake")?;
            }
            Ok(Some(Ok(Message::Binary(_) | Message::Pong(_) | Message::Frame(_)))) => {}
        }
    }
}

fn handshake_from_message(text: &str, expected: RemoteProtocol) -> Result<Handshake> {
    match parse_server_message(text)? {
        ServerMessage::Started {
            protocol_version,
            flow_id,
            credit,
        } if protocol_version == expected.version() && credit > 0 && !flow_id.is_empty() => {
            Ok(Handshake::Negotiated {
                protocol: expected,
                flow_id,
                credit,
            })
        }
        ServerMessage::Started {
            protocol_version,
            credit,
            ..
        } => Err(anyhow!(
            "remote stream negotiated invalid protocol version {protocol_version} or credit {credit}"
        )),
        ServerMessage::Error { .. } => Ok(Handshake::RetryLegacy),
        _ => Err(anyhow!(
            "remote stream sent data before completing the v2 handshake"
        )),
    }
}

impl RemoteProtocol {
    fn version(self) -> u8 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
            Self::V3 => 3,
        }
    }
}

fn commit_message(protocol: RemoteProtocol, segment_index: u64) -> String {
    match protocol {
        RemoteProtocol::V1 => "CommitSegment".to_string(),
        RemoteProtocol::V2 | RemoteProtocol::V3 => serde_json::json!({
            "type": "CommitSegment",
            "segment_index": segment_index,
        })
        .to_string(),
    }
}

fn stream_endpoint(endpoint: &str) -> Result<Url> {
    let normalized = crate::remote::RemoteClient::validate_endpoint(endpoint)?;
    let mut url = Url::parse(&normalized).context("invalid remote API endpoint")?;
    let websocket_scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        _ => return Err(anyhow!("remote API endpoint must use http or https")),
    };
    url.set_scheme(websocket_scheme)
        .map_err(|_| anyhow!("failed to convert API endpoint to WebSocket URL"))?;
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow!("API endpoint cannot be used as a base URL"))?;
        path.pop_if_empty();
        path.push("v1");
        path.push("stream");
    }
    Ok(url)
}

fn parse_server_message(text: &str) -> Result<ServerMessage> {
    serde_json::from_str(text).context("failed to decode remote streaming response")
}

fn encode_pcm_packet(samples: &[f32], format: StreamingPcmFormat) -> Result<Vec<u8>> {
    let bytes_per_sample = match format {
        StreamingPcmFormat::S16le => std::mem::size_of::<i16>(),
        StreamingPcmFormat::F32le => std::mem::size_of::<f32>(),
    };
    let mut packet = Vec::with_capacity(samples.len() * bytes_per_sample);
    for sample in samples {
        if !sample.is_finite() {
            return Err(anyhow!("microphone audio contains a non-finite sample"));
        }
        match format {
            StreamingPcmFormat::S16le => {
                let quantized = (sample.clamp(-1.0, 1.0) * 32_768.0)
                    .round()
                    .clamp(f32::from(i16::MIN), f32::from(i16::MAX))
                    as i16;
                packet.extend_from_slice(&quantized.to_le_bytes());
            }
            StreamingPcmFormat::F32le => packet.extend_from_slice(&sample.to_le_bytes()),
        }
    }
    Ok(packet)
}

fn resample_linear(samples: &[f32], source_rate: u32, target_rate: u32) -> Result<Vec<f32>> {
    if source_rate == 0 || target_rate == 0 {
        return Err(anyhow!("audio sample rate must be greater than zero"));
    }
    if samples.is_empty() || source_rate == target_rate {
        return Ok(samples.to_vec());
    }
    let output_len =
        ((samples.len() as u128 * target_rate as u128).div_ceil(source_rate as u128)) as usize;
    let scale = source_rate as f64 / target_rate as f64;
    let mut output = Vec::with_capacity(output_len);
    for index in 0..output_len {
        let source = index as f64 * scale;
        let left = (source.floor() as usize).min(samples.len() - 1);
        let right = (left + 1).min(samples.len() - 1);
        let fraction = (source - left as f64) as f32;
        output.push(samples[left] + (samples[right] - samples[left]) * fraction);
    }
    Ok(output)
}

fn opus_frames(samples: &[f32]) -> Vec<Vec<f32>> {
    samples
        .chunks(OPUS_FRAME_SAMPLES)
        .map(|chunk| {
            let mut frame = vec![0.0; OPUS_FRAME_SAMPLES];
            frame[..chunk.len()].copy_from_slice(chunk);
            frame
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start_json(message: Message) -> serde_json::Value {
        let Message::Text(text) = message else {
            panic!("expected text Start message");
        };
        serde_json::from_str(text.as_str()).expect("decode Start message")
    }

    #[test]
    fn constructs_stream_endpoint_without_dropping_base_path() {
        assert_eq!(
            stream_endpoint("https://example.test/shadoword/")
                .unwrap()
                .as_str(),
            "wss://example.test/shadoword/v1/stream"
        );
    }

    #[test]
    fn parses_v2_handshake_credit_acceptance_and_indexed_partial() {
        assert!(matches!(
            handshake_from_message(
                r#"{"type":"Started","protocol_version":2,"flow_id":"flow-7","credit":4}"#,
                RemoteProtocol::V2,
            )
            .unwrap(),
            Handshake::Negotiated {
                protocol: RemoteProtocol::V2,
                flow_id,
                credit,
            } if flow_id == "flow-7" && credit == 4
        ));
        assert_eq!(
            parse_server_message(
                r#"{"type":"Accepted","segment_index":3,"outstanding":2,"remaining_credit":2}"#
            )
            .unwrap(),
            ServerMessage::Accepted {
                segment_index: 3,
                outstanding: 2,
                remaining_credit: 2,
                audio_duration_ms: None,
                sample_rate: None,
            }
        );
        assert_eq!(
            parse_server_message(
                r#"{"type":"Accepted","segment_index":4,"outstanding":1,"remaining_credit":3,"audio_duration_ms":780,"sample_rate":16000}"#
            )
            .unwrap(),
            ServerMessage::Accepted {
                segment_index: 4,
                outstanding: 1,
                remaining_credit: 3,
                audio_duration_ms: Some(780),
                sample_rate: Some(16_000),
            }
        );
        assert!(matches!(
            parse_server_message(
                r#"{"type":"Partial","segment_index":3,"text":"hello","elapsed_ms":42,"engine":"whisper"}"#
            )
            .unwrap(),
            ServerMessage::Partial { segment_index: 3, .. }
        ));
        assert_eq!(
            commit_message(RemoteProtocol::V2, 3),
            r#"{"segment_index":3,"type":"CommitSegment"}"#
        );
    }

    #[test]
    fn v1_fallback_uses_lockstep_commit_and_rejects_invalid_v2_credit() {
        assert_eq!(
            commit_message(RemoteProtocol::V1, 9),
            "CommitSegment".to_string()
        );
        assert!(handshake_from_message(
            r#"{"type":"Started","protocol_version":2,"flow_id":"flow","credit":0}"#,
            RemoteProtocol::V2,
        )
        .is_err());
        assert!(matches!(
            handshake_from_message(
                r#"{"type":"Error","code":"bad","message":"old"}"#,
                RemoteProtocol::V3,
            )
            .unwrap(),
            Handshake::RetryLegacy
        ));
    }

    #[test]
    fn resamples_and_pads_opus_frames() {
        let input = vec![0.25; 16_000];
        let output = resample_linear(&input, 16_000, 48_000).unwrap();
        assert_eq!(output.len(), 48_000);
        let frames = opus_frames(&output[..1_000]);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].len(), OPUS_FRAME_SAMPLES);
        assert!(frames[1][40..].iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn encodes_selectable_pcm_precision() {
        assert_eq!(
            encode_pcm_packet(&[0.5, -0.25, 1.0, -1.0], StreamingPcmFormat::S16le)
                .expect("encode s16 PCM"),
            [0x00, 0x40, 0x00, 0xe0, 0xff, 0x7f, 0x00, 0x80]
        );
        assert_eq!(
            encode_pcm_packet(&[0.5, -0.25], StreamingPcmFormat::F32le).expect("encode f32 PCM"),
            [0.5_f32, -0.25_f32]
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect::<Vec<_>>()
        );
        assert!(encode_pcm_packet(&[f32::NAN], StreamingPcmFormat::S16le).is_err());
    }

    #[tokio::test]
    async fn v3_streams_raw_pcm_without_opus_commits() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept v3 socket");
            let mut socket = tokio_tungstenite::accept_async(stream)
                .await
                .expect("accept v3 websocket");
            let start = start_json(
                socket
                    .next()
                    .await
                    .expect("v3 Start message")
                    .expect("read v3 Start"),
            );
            assert_eq!(start["sample_rate"], 16_000);
            assert_eq!(start["audio_format"], "pcm_s16le");
            socket
                .send(Message::text(
                    r#"{"type":"Started","protocol_version":3,"flow_id":"raw-flow","credit":4}"#,
                ))
                .await
                .expect("send Started");

            let packet = socket
                .next()
                .await
                .expect("raw PCM packet")
                .expect("read raw PCM packet");
            let Message::Binary(packet) = packet else {
                panic!("expected binary raw PCM packet");
            };
            assert_eq!(packet.as_ref(), [0x00, 0x40, 0x00, 0xe0]);
            let finish = socket
                .next()
                .await
                .expect("Finish message")
                .expect("read Finish");
            assert_eq!(finish.into_text().expect("Finish text"), "Finish");
            socket
                .send(Message::text(r#"{"type":"Done","text":"","segments":[]}"#))
                .await
                .expect("send Done");
        });

        let mut remote = RemoteStream::connect_with_pcm_format(
            &format!("http://{address}"),
            None,
            16_000,
            StreamingPcmFormat::S16le,
        )
        .await
        .expect("v3 connection");
        assert_eq!(remote.protocol(), RemoteProtocol::V3);
        remote
            .send_samples(&[0.5, -0.25])
            .await
            .expect("send raw PCM");
        remote.finish_request().await.expect("send Finish");
        assert!(matches!(
            remote.next_event().await.expect("receive Done"),
            RemoteEvent::Done(_)
        ));
        remote.close().await;
        server.await.expect("test server task");
    }

    #[tokio::test]
    async fn slow_modern_handshakes_close_and_reconnect_with_explicit_v1_start() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let server = tokio::spawn(async move {
            let (v3_stream, _) = listener.accept().await.expect("accept v3 socket");
            let mut v3 = tokio_tungstenite::accept_async(v3_stream)
                .await
                .expect("accept v3 websocket");
            let v3_start = start_json(
                v3.next()
                    .await
                    .expect("v3 Start message")
                    .expect("read v3 Start"),
            );
            assert_eq!(v3_start["protocol_version"], 3);
            assert_eq!(v3_start["audio_format"], "pcm_f32le");

            let (v2_stream, _) = listener.accept().await.expect("accept v2 socket");
            let mut v2 = tokio_tungstenite::accept_async(v2_stream)
                .await
                .expect("accept v2 websocket");
            let v2_start = start_json(
                v2.next()
                    .await
                    .expect("v2 Start message")
                    .expect("read v2 Start"),
            );
            assert_eq!(v2_start["protocol_version"], 2);

            let (fallback_stream, _) = listener.accept().await.expect("accept v1 socket");
            let mut fallback = tokio_tungstenite::accept_async(fallback_stream)
                .await
                .expect("accept v1 websocket");
            start_json(
                fallback
                    .next()
                    .await
                    .expect("v1 Start message")
                    .expect("read v1 Start"),
            )
        });

        let remote = RemoteStream::connect(&format!("http://{address}"), None, 16_000)
            .await
            .expect("fallback connection");
        assert_eq!(remote.protocol(), RemoteProtocol::V1);
        remote.close().await;
        let fallback_start = server.await.expect("test server task");
        assert_eq!(fallback_start["type"], "Start");
        assert!(fallback_start.get("protocol_version").is_none());
    }
}
