use heapless::Vec;

#[macro_use]
pub mod macros;
pub mod application_layer;
mod compression;
mod encryption;
mod framing;
pub mod phy;

use lora_phy::mod_params::RadioError;
use phy::VLPPhy;

use self::framing::{Flags, FramingError, Packet};
use defmt::{info, warn};

pub const MAX_PAYLOAD_LENGTH: usize = 222;

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
    pub encryption: bool,
    pub compression: bool,
    pub reliability: bool,
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

#[derive(defmt::Format, Debug, PartialEq)]
pub enum VLPError {
    IllegalPriority(Priority),
    Phy(RadioError),
    Framing(FramingError),
    InvalidSeqnum,
    SessionReset,
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
    pub prio: Priority,
    params: SocketParams,
    pub next_seqnum: u16,
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

        _self.do_establish().await;

        _self
    }

    async fn do_establish(&mut self) {
        self.next_seqnum = 0;
        let estab = Packet {
            flags: Flags::establish_with_params(&self.params),
            seqnum: self.next_seqnum,
            payload: None,
        };
        self.next_seqnum += 1;

        if self.params.reliability {
            //We are sending a handshake packet, SessionReset will never occur here
            let _ = self.reliable_tx(&estab).await;
        } else {
            self.phy.tx(&estab.serialize()[..]).await;
        }

        self.state = ConnectionState::Established;
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
            match Packet::deserialize(_self.phy.rx().await?.1) {
                Ok(packet) => {
                    log_info!("Received packet {:?}", packet);
                    // Normal path
                    if packet.flags.contains(Flags::ESTABLISH) {
                        log_info!("Normal path. cxn established");
                        _self.params.compression = packet.flags.contains(Flags::COMPRESSION);
                        _self.params.encryption = packet.flags.contains(Flags::ENCRYPTION);
                        _self.params.reliability = packet.flags.contains(Flags::RELIABLE);
                        break;
                    } else {
                        log_info!("Anomaly path. Sending RST");
                        // Anomaly: remote party believes a session to already be established.
                        let rst = Packet {
                            flags: Flags::RST,
                            seqnum: packet.seqnum + 1,
                            payload: None,
                        };

                        // Send RST packet, and await new handshake from remote party.
                        _self.phy.tx(&rst.serialize()[..]).await;
                        _self.phy.reset_frequency();
                    }
                }
                Err(_) => continue,
            }
        }

        _self.phy.increment_frequency();

        if _self.params.reliability {
            log_info!("Sending ack");
            let ack = Packet {
                flags: Flags::ACK,
                seqnum: _self.next_seqnum,
                payload: None,
            };
            _self.phy.tx(&ack.serialize()[..]).await;
            _self.next_seqnum = _self.next_seqnum.wrapping_add(1);
            _self.phy.increment_frequency();
        }

        _self.state = ConnectionState::Established;
        Ok(_self)
    }

    pub async fn transmit(
        &mut self,
        payload: Vec<u8, MAX_PAYLOAD_LENGTH>,
    ) -> Result<Option<Vec<u8, MAX_PAYLOAD_LENGTH>>, VLPError> {
        if self.prio == Priority::Listener {
            return Err(VLPError::IllegalPriority(self.prio));
        }
        let mut packet = Packet {
            flags: Flags::PSH,
            seqnum: self.next_seqnum,
            payload: Some(payload),
        };

        self.next_seqnum = self.next_seqnum.wrapping_add(1);

        if self.params.reliability {
            loop {
                match self.reliable_tx(&packet).await {
                    Ok(packet) => {
                        // TODO: Is this a good way to handle PSH|ACK from the passive party?
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
                    Err(e) => {
                        if e == VLPError::SessionReset {
                            self.do_establish().await;
                            packet.seqnum = self.next_seqnum;
                            self.next_seqnum = self.next_seqnum.wrapping_add(1);
                        }
                    }
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

        let mut packet = Packet {
            flags: Flags::HANDOFF,
            seqnum: self.next_seqnum,
            payload: None,
        };

        self.next_seqnum = self.next_seqnum.wrapping_add(1);

        loop {
            match self.reliable_tx(&packet).await {
                Ok(_) => {
                    self.prio = Priority::Listener;
                    return Ok(());
                }
                Err(e) => {
                    if e == VLPError::SessionReset {
                        self.do_establish().await;
                        packet.seqnum = self.next_seqnum;
                        self.next_seqnum = self.next_seqnum.wrapping_add(1);
                    }
                }
            }
        }
    }

    pub async fn receive(&mut self) -> Result<Option<Vec<u8, MAX_PAYLOAD_LENGTH>>, VLPError> {
        if self.prio == Priority::Driver {
            return Err(VLPError::IllegalPriority(self.prio));
        }

        let packet = Packet::deserialize(self.phy.rx().await?.1)?;
        self.phy.increment_frequency();
        log_info!("recvd {:?}", packet);
        if packet.flags.contains(Flags::HANDOFF) {
            // HANDOFF must be ACKed, regardless of reliability of transport
            let ack = Packet {
                flags: Flags::ACK | Flags::HANDOFF,
                seqnum: packet.seqnum + 1,
                payload: None,
            };
            self.phy.tx(&ack.serialize()[..]).await;
            self.phy.increment_frequency();
            self.next_seqnum = packet.seqnum.wrapping_add(2);
            self.prio = Priority::Driver;
            log_info!("Priority changed. I'm in charge");
        } else if self.params.reliability
            && (packet.seqnum == self.next_seqnum || packet.seqnum == self.next_seqnum - 2)
        {
            let ack = Packet {
                flags: Flags::ACK,
                seqnum: self.next_seqnum + 1,
                payload: None,
            };
            self.phy.tx(&ack.serialize()[..]).await;
            self.phy.increment_frequency();
            self.next_seqnum = self.next_seqnum.wrapping_add(2);
            log_info!("Ack sent");
        } else if self.params.reliability && packet.seqnum != self.next_seqnum {
            return Err(VLPError::InvalidSeqnum);
        } else if !self.params.reliability && packet.seqnum > self.next_seqnum {
            log_warn!(
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

    async fn reliable_tx(&mut self, packet: &Packet) -> Result<Packet, VLPError> {
        log_info!("Reliable send of packet {:?}", packet);
        let packet = packet.serialize();
        self.phy.tx(&packet[..]).await;
        self.phy.increment_frequency();

        loop {
            match self.phy.rx_with_timeout(2000).await {
                Ok(resp) => {
                    match Packet::deserialize(resp.1) {
                        Ok(recv) => {
                            // Validate seqnum against record. Re-tx if mismatch
                            if recv.flags.contains(Flags::ACK) && recv.seqnum == self.next_seqnum {
                                log_info!("ACK recvd");
                                self.next_seqnum = self.next_seqnum.wrapping_add(1);
                                self.phy.increment_frequency();
                                return Ok(recv);
                            } else if recv.flags.contains(Flags::RST) {
                                log_info!("RST recvd.");
                                // Can't handle re-establishing here because recursion
                                self.phy.reset_frequency();
                                return Err(VLPError::SessionReset);
                            } else {
                                log_info!("retx");
                                self.phy.tx(&packet[..]).await;
                            }
                        }
                        Err(_) => {
                            log_info!("retx (deser error)");
                            self.phy.tx(&packet[..]).await;
                        }
                    }
                }
                Err(RadioError::ReceiveTimeout) => {
                    log_info!("retx (timeout)");
                    self.phy.tx(&packet[..]).await;
                }
                Err(e) => {
                    panic!("{:?}", e);
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

    use super::{*, phy::RadioReceiveInfo};

    #[inline(never)]
    #[no_mangle]
    fn _defmt_acquire() {}

    #[inline(never)]
    #[no_mangle]
    fn _defmt_release() {}

    #[inline(never)]
    #[no_mangle]
    fn _defmt_flush() {}

    #[inline(never)]
    #[no_mangle]
    fn _defmt_write(_: &[u8]) {}

    #[inline(never)]
    #[no_mangle]
    fn _defmt_timestamp(_: defmt::Formatter<'_>) {}

    #[inline(never)]
    #[no_mangle]
    fn _defmt_panic() -> ! {
        loop {}
    }

    struct MockPhy {
        tag: String,
        channel_a: Channel<NoopRawMutex, Vec<u8, MAX_PAYLOAD_LENGTH>, 2>,
        channel_b: Channel<NoopRawMutex, Vec<u8, MAX_PAYLOAD_LENGTH>, 2>,
    }

    impl MockPhy {
        fn new(tag: &str) -> Self {
            Self {
                tag: tag.to_string(),
                channel_a: Channel::new(),
                channel_b: Channel::new(),
            }
        }

        fn get_participants(&self) -> (MockPhyParticipant, MockPhyParticipant) {
            (
                MockPhyParticipant {
                    tag: self.tag.clone(),
                    is_a: true,
                    sender: self.channel_a.sender(),
                    receiver: self.channel_b.receiver(),
                },
                MockPhyParticipant {
                    tag: self.tag.clone(),
                    is_a: false,
                    sender: self.channel_b.sender(),
                    receiver: self.channel_a.receiver(),
                },
            )
        }
    }

    struct MockPhyParticipant<'a> {
        tag: String,
        is_a: bool,
        sender: Sender<'a, NoopRawMutex, Vec<u8, MAX_PAYLOAD_LENGTH>, 2>,
        receiver: Receiver<'a, NoopRawMutex, Vec<u8, MAX_PAYLOAD_LENGTH>, 2>,
    }

    impl<'a> VLPPhy for MockPhyParticipant<'a> {
        async fn tx(&mut self, payload: &[u8]) {
            if self.is_a {
                println!("{}: A --{:02X?}-> B", self.tag, payload);
            } else {
                println!("{}: A <-{:02X?}-- B", self.tag, payload);
            }
            self.sender.send(Vec::from_slice(payload).unwrap()).await;
        }

        async fn rx(&mut self) -> Result<(RadioReceiveInfo,Vec<u8, MAX_PAYLOAD_LENGTH>), RadioError> {
            let received=self.receiver.recv().await;
            let info = RadioReceiveInfo {
                rssi: 0,
                snr: 0,
                len: received.len() as u8,
            };
            Ok((info,received))
        }

        async fn rx_with_timeout(
            &mut self,
            _timeout_ms: u32,
        ) -> Result<(RadioReceiveInfo,Vec<u8, MAX_PAYLOAD_LENGTH>), RadioError> {
            let rxfut = self.rx();
            pin_mut!(rxfut);
            match select(Delay::new(Duration::from_millis(_timeout_ms as u64)), rxfut).await {
                Either::Left(_) => Err(RadioError::ReceiveTimeout),
                Either::Right((x, _)) => x,
            }
        }

        fn set_frequency(&mut self, frequency: u32) {}

        fn increment_frequency(&mut self) {}

        fn reset_frequency(&mut self) {}

        fn set_output_power(&mut self, _power: i32) {}
    }

    // BEGIN RELIABLE TRANSPORT TESTS
    #[futures_test::test]
    async fn establish() {
        let mock_phy = MockPhy::new("establish");
        let (part_a, part_b) = mock_phy.get_participants();

        let establish = async {
            let mut _socket = VLPSocket::establish(
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
            let mut _socket = VLPSocket::await_establish(part_b).await.unwrap();
            println!("await_establish success!");
        };

        let join_fut = join(establish, await_establish);
        pin_mut!(join_fut);
        if let Either::Left(_) = select(Delay::new(Duration::from_millis(100)), join_fut).await {
            panic!("timeout")
        }
    }

    #[futures_test::test]
    async fn establish_retransmit() {
        let mock_phy = MockPhy::new("establish_retransmit");
        let (part_a, part_b) = mock_phy.get_participants();

        let estab = async {
            let _txsock = VLPSocket::establish(
                part_a,
                SocketParams {
                    encryption: false,
                    compression: false,
                    reliability: true,
                },
            )
            .await;

            println!("Unreachable");
        };

        pin_mut!(estab);
        if let Either::Left(_) = select(Delay::new(Duration::from_millis(2500)), estab).await {
            part_b.receiver.recv().await;
            println!("First packet received");
            part_b.receiver.recv().await;
            println!("Second packet received");
        }
    }

    #[futures_test::test]
    async fn establish_and_transmit() {
        let mock_phy = MockPhy::new("establish_and_transmit");
        let (part_a, part_b) = mock_phy.get_participants();

        let tx = VLPSocket::establish(
            part_a,
            SocketParams {
                encryption: false,
                compression: false,
                reliability: true,
            },
        );
        pin_mut!(tx);

        let rx = VLPSocket::await_establish(part_b);
        pin_mut!(rx);

        let (mut tx, rx) = join(tx, rx).await;
        match rx {
            Ok(mut rx) => {
                let txfut = tx.transmit(packet![0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]);
                let rxfut = rx.receive();

                let res = join(txfut, rxfut).await;
                match res {
                    (Ok(a), Ok(b)) => {
                        assert!(a.is_none());
                        assert!(b.is_some());
                        assert_eq!(
                            &b.unwrap()[..],
                            &[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]
                        );
                    }
                    _ => panic!("{:?}", res),
                }
            }
            Err(e) => panic!("{:?}", e),
        }
    }

    #[futures_test::test]
    async fn test_session_reset() {
        let mock_phy = MockPhy::new("test_session_reset");
        let (part_a, part_b) = mock_phy.get_participants();

        let tx = VLPSocket::establish(
            part_a,
            SocketParams {
                encryption: false,
                compression: false,
                reliability: true,
            },
        );
        pin_mut!(tx);

        let rx = VLPSocket::await_establish(part_b);
        pin_mut!(rx);

        let (mut tx, rx) = join(tx, rx).await;
        match rx {
            Ok(mut rx) => {
                // Session sanity check
                let (_, buf) = join(tx.transmit(packet![0xab, 0xab, 0xab]), rx.receive()).await;
                assert_eq!(buf, Ok(Some(packet![0xab, 0xab, 0xab])));

                let newrx = async {
                    if let Ok(mut rx) = VLPSocket::await_establish(rx.phy).await {
                        let buf = rx.receive().await;
                        assert_eq!(buf, Ok(Some(packet![0xff, 0xff, 0xff])));
                    }
                };
                pin_mut!(newrx);

                let _ = join(tx.transmit(packet![0xff, 0xff, 0xff]), newrx).await;
            }
            Err(e) => panic!("{:?}", e),
        }
    }

    #[futures_test::test]
    async fn establish_and_handoff() {
        let mock_phy = MockPhy::new("establish_and_handoff");
        let (part_a, part_b) = mock_phy.get_participants();

        let tx = VLPSocket::establish(
            part_a,
            SocketParams {
                encryption: false,
                compression: false,
                reliability: true,
            },
        );
        pin_mut!(tx);

        let rx = VLPSocket::await_establish(part_b);
        pin_mut!(rx);

        let (mut tx, rx) = join(tx, rx).await;
        match rx {
            Ok(mut rx) => {
                assert_eq!(tx.prio, Priority::Driver);
                assert_eq!(rx.prio, Priority::Listener);
                let txfut = tx.handoff();
                let rxfut = rx.receive();

                let res = join(txfut, rxfut).await;
                match res {
                    (Ok(_), Ok(_)) => {
                        assert_eq!(tx.prio, Priority::Listener);
                        assert_eq!(rx.prio, Priority::Driver);
                    }
                    _ => panic!("{:?}", res),
                }
            }
            Err(e) => panic!("{:?}", e),
        }
    }
    // END RELIABLE TRANSPORT TESTS

    #[futures_test::test]
    async fn establish_and_transmit_unreliable() {
        let mock_phy = MockPhy::new("establish_and_transmit_unreliable");
        let (part_a, _) = mock_phy.get_participants();

        let tx = VLPSocket::establish(
            part_a,
            SocketParams {
                encryption: false,
                compression: false,
                reliability: false,
            },
        );
        pin_mut!(tx);

        match select(Delay::new(Duration::from_millis(100)), tx).await {
            Either::Right((mut tx, _)) => {
                let txfut = tx.transmit(packet![0xff, 0xff, 0xff]);
                pin_mut!(txfut);
                if let Either::Left(_) = select(Delay::new(Duration::from_millis(100)), txfut).await
                {
                    panic!("Timeout transmit");
                }
            }
            Either::Left(_) => panic!("Timeout"),
        }
    }

    #[futures_test::test]
    async fn establish_and_handoff_unreliable() {
        let mock_phy = MockPhy::new("establish_and_handoff_unreliable");
        let (part_a, part_b) = mock_phy.get_participants();

        let tx = VLPSocket::establish(
            part_a,
            SocketParams {
                encryption: false,
                compression: false,
                reliability: false,
            },
        );
        pin_mut!(tx);

        let rx = VLPSocket::await_establish(part_b);
        pin_mut!(rx);

        let (mut tx, rx) = join(tx, rx).await;
        match rx {
            Ok(mut rx) => {
                assert_eq!(tx.prio, Priority::Driver);
                assert_eq!(rx.prio, Priority::Listener);
                let txfut = tx.handoff();
                let rxfut = rx.receive();

                let res = join(txfut, rxfut).await;
                match res {
                    (Ok(_), Ok(_)) => {
                        assert_eq!(tx.prio, Priority::Listener);
                        assert_eq!(rx.prio, Priority::Driver);
                    }
                    _ => panic!("{:?}", res),
                }
            }
            Err(e) => panic!("{:?}", e),
        }
    }
}
