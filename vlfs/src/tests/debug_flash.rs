use std::time::Duration;

use crate::Flash;
use futures::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::to_string;
use tokio::{
    net::{TcpListener, TcpStream},
    time::timeout,
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

const SIZE: u32 = 262144 * 256;

pub struct DebugFlash {
    stream: WebSocketStream<TcpStream>,
}

impl DebugFlash {
    pub async fn new() -> Self {
        let addr = "0.0.0.0:19000";
        let try_socket = TcpListener::bind(addr).await;
        let listener = try_socket.expect("Failed to bind");
        log_info!("Listening on: {}", addr);

        loop {
            log_info!("Waiting for connection");
            let stream = timeout(Duration::from_millis(500), async {
                let (stream, _) = listener.accept().await.unwrap();
                let stream = tokio_tungstenite::accept_async(stream).await.unwrap();
                log_info!("Accepted connection");
                stream
            })
            .await;
            if let Ok(stream) = stream {
                return Self { stream };
            }
        }
    }
}

impl Flash for DebugFlash {
    type Error = ();

    async fn size(&self) -> u32 {
        SIZE
    }

    async fn reset(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn erase_sector_4kib(&mut self, address: u32) -> Result<(), Self::Error> {
        let request = EraseRequest {
            typ: "eraseSector4Kib".to_string(),
            address,
        };
        log_info!("Sending: {:?}", request);
        self.stream
            .send(Message::text(to_string(&request).unwrap()))
            .await
            .unwrap();
        log_info!("Waiting for response");
        self.stream.next().await.unwrap().unwrap();
        log_info!("Response received");
        Ok(())
    }

    async fn erase_block_32kib(&mut self, address: u32) -> Result<(), Self::Error> {
        let request = EraseRequest {
            typ: "eraseBlock32Kib".to_string(),
            address,
        };
        self.stream
            .send(Message::text(to_string(&request).unwrap()))
            .await
            .unwrap();
        self.stream.next().await.unwrap().unwrap();
        Ok(())
    }

    async fn erase_block_64kib(&mut self, address: u32) -> Result<(), Self::Error> {
        let request = EraseRequest {
            typ: "eraseBlock64Kib".to_string(),
            address,
        };
        self.stream
            .send(Message::text(to_string(&request).unwrap()))
            .await
            .unwrap();
        self.stream.next().await.unwrap().unwrap();
        Ok(())
    }

    async fn read_4kib<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        let request = ReadRequest {
            typ: "read".to_string(),
            address,
            length: read_length as u32,
        };
        self.stream
            .send(Message::text(to_string(&request).unwrap()))
            .await
            .unwrap();
        let response = self.stream.next().await.unwrap().unwrap();
        if let Message::Text(text) = response {
            let bytes: Vec<u8> = serde_json::from_str(&text).unwrap();
            (&mut read_buffer[5..(5 + read_length)]).copy_from_slice(&bytes);
            return Ok(&read_buffer[5..(5 + read_length)]);
        } else {
            panic!("Unexpected response: {:?}", response);
        }
    }

    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        assert!(write_buffer.len() == 256 + 5);
        let request = WriteRequest {
            typ: "write256b".to_string(),
            address,
            data: write_buffer[5..].to_vec(),
        };
        self.stream
            .send(Message::text(to_string(&request).unwrap()))
            .await
            .unwrap();
        self.stream.next().await.unwrap().unwrap();
        Ok(())
    }
}

#[derive(Serialize, Debug)]
struct EraseRequest {
    #[serde(rename = "type")]
    typ: String,
    address: u32,
}

#[derive(Serialize, Debug)]
struct ReadRequest {
    #[serde(rename = "type")]
    typ: String,
    address: u32,
    length: u32,
}

#[derive(Serialize, Debug)]
struct WriteRequest {
    #[serde(rename = "type")]
    typ: String,
    address: u32,
    data: Vec<u8>,
}

#[cfg(feature = "tests_use_debug_flash")]
#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn read_write() {
        env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init()
            .unwrap();

        let mut flash = DebugFlash::new().await;
        flash.erase_sector_4kib(0x8000).await.unwrap();
        let mut buffer = [0u8; 256 + 5];
        for i in 0u8..10 {
            buffer[5 + i as usize] = i;
        }
        flash.write_256b(0, &mut buffer).await.unwrap();
        flash.read_4kib(5, 10, &mut buffer).await.unwrap();
        info!("{:02X?}", &buffer[5..15]);
    }
}
