use std::time::Duration;

use firmware_common::driver::buzzer::Buzzer;
use rodio::{OutputStream, OutputStreamHandle, Source};
use tokio::time::sleep;

pub struct SpeakerBuzzer {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
}

impl SpeakerBuzzer {
    pub fn new() -> Self {
        let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        Self {
            _stream: stream,
            stream_handle,
        }
    }
}

impl Buzzer for SpeakerBuzzer {
    async fn play(&mut self, frequency: u32, duration_ms: f64) {
        let duration = Duration::from_secs_f64(duration_ms / 1000.0);
        let source = rodio::source::SineWave::new(frequency as f32).take_duration(duration);

        self.stream_handle
            .play_raw(source.convert_samples())
            .unwrap();
        sleep(duration).await;
    }
}
