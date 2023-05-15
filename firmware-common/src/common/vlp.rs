use heapless::Vec;

#[macro_use]
mod macros;
mod compression;
mod encryption;
mod framing;
mod phy;

use lora_phy::{mod_params::RadioError, mod_traits::RadioKind, LoRa};
use phy::VLPPhy;

use self::framing::{Flags, FramingError, Packet};
use defmt::warn;

const MAX_PAYLOAD_LENGTH: usize = 222;

/// The current pVriority state of a given VLP party.
/// As LoRa is half duplex, conflicts are avoided through coarse-grain timeslicing through the priority mechanism
#[derive(Copy, Clone, Debug, PartialEq, Eq, defmt::Format)]
pub enum Priority {
    /// Local party is actively driving the socket. A driving party may send any category of packet they desire.
    Driver,
    /// Local party is the listening party. Listening parties may only send packets with ACK=1 (reliable transports only).
    /// If the transport is unreliable, listening parties may not send any packets, unless a packet with HANDOFF=1 is received.
    Listener,
}

#[derive(Copy, Clone, Debug, Default, defmt::Format)]
pub struct SocketParams {
    encryption: bool,
    compression: bool,
    reliability: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, defmt::Format)]
pub enum ConnectionState {
    /// Local party is disconnected.
    Disconnected,
    /// Local party is in the process of establishing a connection, with the given priority
    Establishing,
    /// Local party has established a connection, and has the specified priority for this connection
    Established,
    /// Local party has sent a HANDOFF packet to the remote party, and is awaiting their acknowledgement
    HandingOff,
    /// Local party is awaiting the ACK for a packet where HANDOFF=0. Reliable transport only.
    AwaitingAck,
}

#[derive(defmt::Format, Debug)]
pub enum VLPError {
    IllegalPriority(Priority),
    Phy(RadioError),
    Framing(FramingError),
    InvalidSeqnum,
}

impl From<RadioError> for VLPError {
    fn from(value: RadioError) -> Self {
        VLPError::Phy(value)
    }
}

impl From<FramingError> for VLPError {
    fn from(value: FramingError) -> Self {
        VLPError::Framing(value)
    }
}

pub struct VLPSocket<P: VLPPhy> {
    //TODO: Encryption, Compression
    phy: P,
    state: ConnectionState,
    prio: Priority,
    params: SocketParams,
    next_seqnum: u16,
}

impl<P: VLPPhy> VLPSocket<P> {
    pub async fn establish(phy: P, params: SocketParams) -> VLPSocket<P> {
        let mut _self = VLPSocket {
            phy,
            state: ConnectionState::Establishing,
            prio: Priority::Driver,
            params,
            next_seqnum: 0,
        };

        let estab = Packet {
            flags: Flags::establish_with_params(&_self.params),
            seqnum: _self.next_seqnum,
            payload: None,
        };
        _self.next_seqnum += 1;

        if _self.params.reliability {
            let _ = _self.reliable_tx(&estab).await;
        } else {
            _self.phy.tx(&estab.serialize()[..]).await;
        }

        _self.state = ConnectionState::Established;

        _self
    }

    pub async fn await_establish(phy: P) -> Result<VLPSocket<P>, VLPError> {
        let mut _self = VLPSocket {
            phy,
            state: ConnectionState::Disconnected,
            prio: Priority::Listener,
            params: SocketParams::default(),
            next_seqnum: 1,
        };

        loop {
            match Packet::deserialize(_self.phy.rx().await?) {
                Ok(packet) => {
                    if packet.flags.contains(Flags::ESTABLISH) {
                        _self.params.compression = packet.flags.contains(Flags::COMPRESSION);
                        _self.params.encryption = packet.flags.contains(Flags::ENCRYPTION);
                        _self.params.reliability = packet.flags.contains(Flags::RELIABLE);
                        break;
                    }
                }
                Err(_) => continue,
            }
        }

        if _self.params.reliability {
            let ack = Packet {
                flags: Flags::ACK,
                seqnum: _self.next_seqnum,
                payload: None,
            };
            _self.phy.tx(&ack.serialize()[..]).await;
            _self.next_seqnum = _self.next_seqnum.wrapping_add(1);
        }

        _self.state = ConnectionState::Established;
        Ok(_self)
    }

    pub async fn transmit(
        &mut self,
        payload: Vec<u8, 222>,
    ) -> Result<Option<Vec<u8, 222>>, VLPError> {
        if self.prio == Priority::Listener {
            return Err(VLPError::IllegalPriority(self.prio));
        }
        let packet = Packet {
            flags: Flags::PSH,
            seqnum: self.next_seqnum,
            payload: Some(payload),
        };

        self.next_seqnum = self.next_seqnum.wrapping_add(1);

        if self.params.reliability {
            loop {
                let packet = self.reliable_tx(&packet).await;
                //TODO: Is this a good way to handle PSH|ACK from the passive party?
                // They'll know that we recv'd their packet if there's no re-tx of the packet they're ACKing
                if packet.flags.contains(Flags::PSH) {
                    let ack = Packet {
                        flags: Flags::ACK,
                        seqnum: self.next_seqnum,
                        payload: None,
                    };
                    self.phy.tx(&ack.serialize()[..]).await;
                    self.next_seqnum = self.next_seqnum.wrapping_add(1);
                    return Ok(packet.payload);
                } else {
                    return Ok(None);
                }
            }
        } else {
            self.phy.tx(&packet.serialize()[..]).await;
        }

        Ok(None)
    }

    pub async fn handoff(&mut self) -> Result<(), VLPError> {
        if self.prio == Priority::Listener {
            return Err(VLPError::IllegalPriority(self.prio));
        }

        let packet = Packet {
            flags: Flags::HANDOFF,
            seqnum: self.next_seqnum,
            payload: None,
        };
        self.next_seqnum = self.next_seqnum.wrapping_add(1);

        self.reliable_tx(&packet).await;
        self.prio = Priority::Listener;
        Ok(())
    }

    pub async fn receive(&mut self) -> Result<Option<Vec<u8, 222>>, VLPError> {
        if self.prio == Priority::Driver {
            return Err(VLPError::IllegalPriority(self.prio));
        }

        let packet = Packet::deserialize(self.phy.rx().await?)?;
        if self.params.reliability
            && (packet.seqnum == self.next_seqnum || packet.seqnum == self.next_seqnum - 2)
        {
            let ack = Packet {
                flags: Flags::ACK,
                seqnum: self.next_seqnum + 1,
                payload: None,
            };
            self.phy.tx(&ack.serialize()[..]).await;
            self.next_seqnum = self.next_seqnum.wrapping_add(2);
        } else if self.params.reliability && packet.seqnum != self.next_seqnum {
            return Err(VLPError::InvalidSeqnum);
        } else if !self.params.reliability && packet.seqnum > self.next_seqnum {
            warn!(
                "VLP: {} packet(s) lost in flight.",
                packet.seqnum - self.next_seqnum
            );
            self.next_seqnum = packet.seqnum + 1; // resynchronize
        }

        if packet.flags.contains(Flags::PSH) {
            Ok(packet.payload)
        } else {
            Ok(None)
        }
    }

    async fn reliable_tx(&mut self, packet: &Packet) -> Packet {
        let packet = packet.serialize();
        self.phy.tx(&packet[..]).await;

        loop {
            match self.phy.rx_with_timeout(2000).await {
                Ok(resp) => {
                    match Packet::deserialize(resp) {
                        Ok(recv) => {
                            // Validate seqnum against record. Re-tx if mismatch
                            if recv.flags.contains(Flags::ACK) && recv.seqnum == self.next_seqnum {
                                self.next_seqnum = self.next_seqnum.wrapping_add(1);
                                return recv;
                            } else {
                                self.phy.tx(&packet[..]).await;
                            }
                        }
                        Err(_) => self.phy.tx(&packet[..]).await,
                    }
                }
                Err(_e) => {
                    self.phy.tx(&packet[..]).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use core::time::Duration;
    use embassy_sync::{
        blocking_mutex::raw::NoopRawMutex,
        channel::{Channel, Receiver, Sender},
    };
    use futures::{
        future::{join, select, Either},
        pin_mut,
    };
    use futures_timer::Delay;

    use super::*;

    struct MockPhy {
        channel_a: Channel<NoopRawMutex, Vec<u8, 222>, 1>,
        channel_b: Channel<NoopRawMutex, Vec<u8, 222>, 1>,
    }

    impl MockPhy {
        fn new() -> Self {
            Self {
                channel_a: Channel::new(),
                channel_b: Channel::new(),
            }
        }

        fn get_participant(&self) -> (MockPhyParticipant, MockPhyParticipant) {
            (
                MockPhyParticipant {
                    is_a: true,
                    sender: self.channel_a.sender(),
                    receiver: self.channel_b.receiver(),
                },
                MockPhyParticipant {
                    is_a: false,
                    sender: self.channel_b.sender(),
                    receiver: self.channel_a.receiver(),
                },
            )
        }
    }

    struct MockPhyParticipant<'a> {
        is_a: bool,
        sender: Sender<'a, NoopRawMutex, Vec<u8, 222>, 1>,
        receiver: Receiver<'a, NoopRawMutex, Vec<u8, 222>, 1>,
    }

    impl<'a> VLPPhy for MockPhyParticipant<'a> {
        async fn tx(&mut self, payload: &[u8]) {
            if self.is_a {
                println!("A --{:02X?}-> B", payload);
            } else {
                println!("A <-{:02X?}-- B", payload);
            }
            self.sender.send(Vec::from_slice(payload).unwrap()).await;
        }

        async fn rx(&mut self) -> Result<Vec<u8, 222>, RadioError> {
            Ok(self.receiver.recv().await)
        }

        async fn rx_with_timeout(&mut self, _timeout_ms: u32) -> Result<Vec<u8, 222>, RadioError> {
            self.rx().await
        }
    }

    #[futures_test::test]
    async fn establish() {
        let mock_phy = MockPhy::new();
        let (mut part_a, mut part_b) = mock_phy.get_participant();

        let establish = async {
            let mut socket = VLPSocket::establish(
                part_a,
                SocketParams {
                    encryption: false,
                    compression: false,
                    reliability: true,
                },
            )
            .await;
            println!("establish success!");
        };

        let await_establish = async {
            let mut socket = VLPSocket::await_establish(part_b).await.unwrap();
            println!("await_establish success!");
        };

        let join_fut = join(establish, await_establish);
        pin_mut!(join_fut);
        if let Either::Left(_) = select(Delay::new(Duration::from_millis(100)), join_fut).await {
            panic!()
        }
    }
}
