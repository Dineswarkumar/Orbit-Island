use std::collections::VecDeque;

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::time::Instant;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator,
        MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_E_DEVICE_INVALIDATED,
        AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX,
        WAVEFORMATEXTENSIBLE,
    };
    use windows::Win32::Media::KernelStreaming::WAVE_FORMAT_EXTENSIBLE;
    use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL};

    const MAX_RING_SAMPLES: usize = 4096;
    const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;

    struct MixFormat(*mut WAVEFORMATEX);

    impl Drop for MixFormat {
        fn drop(&mut self) {
            unsafe { CoTaskMemFree(Some(self.0.cast())) };
        }
    }

    pub struct LoopbackCapture {
        client: IAudioClient,
        capture: IAudioCaptureClient,
        channels: usize,
        sample_rate: u32,
        last_packet: Instant,
    }

    impl LoopbackCapture {
        pub fn new() -> Result<Self, String> {
            unsafe {
                let enumerator: IMMDeviceEnumerator = CoCreateInstance(
                    &MMDeviceEnumerator,
                    None,
                    CLSCTX_ALL,
                )
                .map_err(|error| format!("create audio device enumerator: {error}"))?;
                let device = enumerator
                    .GetDefaultAudioEndpoint(eRender, eConsole)
                    .map_err(|error| format!("get default render endpoint: {error}"))?;
                let client: IAudioClient = device
                    .Activate(CLSCTX_ALL, None)
                    .map_err(|error| format!("activate audio client: {error}"))?;
                let format = MixFormat(
                    client
                        .GetMixFormat()
                        .map_err(|error| format!("get audio mix format: {error}"))?,
                );
                if format.0.is_null() {
                    return Err("audio endpoint returned a null mix format".to_string());
                }

                let wave = &*format.0;
                if !is_float32(wave) {
                    let format_tag = wave.wFormatTag;
                    let bits_per_sample = wave.wBitsPerSample;
                    return Err(format!(
                        "unsupported audio mix format (tag {}, {} bits)",
                        format_tag, bits_per_sample
                    ));
                }
                let channels = wave.nChannels as usize;
                let sample_rate = wave.nSamplesPerSec;
                if channels == 0 || sample_rate == 0 {
                    return Err("audio endpoint returned invalid channel or sample rate".into());
                }

                client
                    .Initialize(
                        AUDCLNT_SHAREMODE_SHARED,
                        AUDCLNT_STREAMFLAGS_LOOPBACK,
                        2_000_000,
                        0,
                        format.0,
                        None,
                    )
                    .map_err(|error| format!("initialize loopback client: {error}"))?;
                let capture: IAudioCaptureClient = client
                    .GetService()
                    .map_err(|error| format!("get audio capture service: {error}"))?;
                client
                    .Start()
                    .map_err(|error| format!("start loopback capture: {error}"))?;

                Ok(Self {
                    client,
                    capture,
                    channels,
                    sample_rate,
                    last_packet: Instant::now(),
                })
            }
        }

        pub fn sample_rate(&self) -> u32 {
            self.sample_rate
        }

        pub fn read_into(&mut self, ring: &mut VecDeque<f32>) -> Result<(), String> {
            loop {
                let packet_frames = unsafe { self.capture.GetNextPacketSize() }
                    .map_err(|error| capture_error("query capture packet", error))?;
                if packet_frames == 0 {
                    break;
                }

                let (data, frames, flags) = unsafe {
                    let mut data = std::ptr::null_mut();
                    let mut frames = 0u32;
                    let mut flags = 0u32;
                    self.capture
                        .GetBuffer(&mut data, &mut frames, &mut flags, None, None)
                        .map_err(|error| capture_error("read capture packet", error))?;
                    (data, frames as usize, flags)
                };

                if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 {
                    for _ in 0..frames {
                        push_bounded(ring, 0.0);
                    }
                } else if !data.is_null() {
                    let interleaved = unsafe {
                        std::slice::from_raw_parts(data.cast::<f32>(), frames * self.channels)
                    };
                    for frame in interleaved.chunks_exact(self.channels) {
                        let mono = frame.iter().copied().sum::<f32>() / self.channels as f32;
                        push_bounded(ring, mono.clamp(-1.0, 1.0));
                    }
                }

                unsafe { self.capture.ReleaseBuffer(frames as u32) }
                    .map_err(|error| capture_error("release capture packet", error))?;
                self.last_packet = Instant::now();
            }

            // WASAPI loopback can legitimately return no packets while output is idle.
            if self.last_packet.elapsed().as_millis() > 150 {
                ring.clear();
                ring.resize(2048, 0.0);
            }
            Ok(())
        }
    }

    impl Drop for LoopbackCapture {
        fn drop(&mut self) {
            let _ = unsafe { self.client.Stop() };
        }
    }

    fn push_bounded(ring: &mut VecDeque<f32>, sample: f32) {
        if ring.len() == MAX_RING_SAMPLES {
            ring.pop_front();
        }
        ring.push_back(sample);
    }

    fn is_float32(format: &WAVEFORMATEX) -> bool {
        if format.wBitsPerSample != 32 {
            return false;
        }
        if format.wFormatTag == WAVE_FORMAT_IEEE_FLOAT {
            return true;
        }
        if format.wFormatTag != WAVE_FORMAT_EXTENSIBLE as u16
            || format.cbSize < (std::mem::size_of::<WAVEFORMATEXTENSIBLE>() - std::mem::size_of::<WAVEFORMATEX>()) as u16
        {
            return false;
        }

        let extended = unsafe { std::ptr::read_unaligned((format as *const WAVEFORMATEX).cast::<WAVEFORMATEXTENSIBLE>()) };
        let sub_format = extended.SubFormat;
        sub_format == windows_core::GUID::from_u128(0x0000000300000010800000aa00389b71)
    }

    fn capture_error(operation: &str, error: windows_core::Error) -> String {
        if error.code() == AUDCLNT_E_DEVICE_INVALIDATED {
            format!("audio output device invalidated during {operation}")
        } else {
            format!("{operation}: {error}")
        }
    }
}

#[cfg(target_os = "windows")]
pub use platform::LoopbackCapture;

#[cfg(not(target_os = "windows"))]
pub struct LoopbackCapture;

#[cfg(not(target_os = "windows"))]
impl LoopbackCapture {
    pub fn new() -> Result<Self, String> {
        Err("WASAPI loopback capture is only available on Windows".into())
    }

    pub fn sample_rate(&self) -> u32 {
        0
    }

    pub fn read_into(&mut self, _ring: &mut VecDeque<f32>) -> Result<(), String> {
        Err("WASAPI loopback capture is only available on Windows".into())
    }
}
