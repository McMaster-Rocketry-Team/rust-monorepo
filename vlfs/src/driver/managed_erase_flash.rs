use core::ops::Range;

use crate::{Flash, Timer};

use super::async_erase_flash::AsyncEraseFlash;

pub struct EraseTune {
    erase_ms_every_write_256b: f64,
}

enum EraseState {
    Idle,
    Erasing { range: Range<u32>, suspended: bool },
}

pub struct ManagedEraseFlash<AF: AsyncEraseFlash, T: Timer> {
    flash: AF,
    erase_state: EraseState,
    tune: EraseTune,
    timer: T,
}

impl<AF: AsyncEraseFlash, T: Timer> ManagedEraseFlash<AF, T> {
    pub fn new(flash: AF, timer: T, tune: EraseTune) -> Self {
        Self {
            flash,
            erase_state: EraseState::Idle,
            tune,
            timer,
        }
    }

    async fn wait_prev_erase(&mut self) -> Result<(), AF::Error> {
        match self.erase_state {
            EraseState::Idle => Ok(()),
            EraseState::Erasing {
                range: _,
                suspended,
            } => {
                if suspended {
                    self.flash.resume_erase().await?;
                }
                self.flash.wait_erase_finish().await?;
                self.erase_state = EraseState::Idle;
                Ok(())
            }
        }
    }
}

impl<AF: AsyncEraseFlash, T: Timer> Flash for ManagedEraseFlash<AF, T> {
    type Error = AF::Error;

    fn size(&self) -> u32 {
        self.flash.size()
    }

    async fn reset(&mut self) -> Result<(), Self::Error> {
        self.flash.reset().await
    }

    async fn erase_sector_4kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.wait_prev_erase().await?;
        self.erase_state = EraseState::Erasing {
            range: address..(address + 4 * 1024),
            suspended: false,
        };
        self.flash.erase_sector_4kib_nb(address).await
    }

    async fn erase_block_32kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.wait_prev_erase().await?;
        self.erase_state = EraseState::Erasing {
            range: address..(address + 32 * 1024),
            suspended: false,
        };
        self.flash.erase_block_32kib_nb(address).await
    }

    async fn erase_block_64kib(&mut self, address: u32) -> Result<(), Self::Error> {
        self.wait_prev_erase().await?;
        self.erase_state = EraseState::Erasing {
            range: address..(address + 64 * 1024),
            suspended: false,
        };
        self.flash.erase_block_64kib_nb(address).await
    }

    async fn read_4kib<'b>(
        &mut self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        if let EraseState::Erasing { range, suspended } = &self.erase_state {
            // if the address is in the range of the current erase operation, wait for it to finish
            if range.contains(&address) {
                self.wait_prev_erase().await?;
            } else if !suspended {
                // if the address is not in the range of the current erase operation, suspend it
                self.flash.suspend_erase().await?;
                self.erase_state = EraseState::Erasing {
                    range: range.clone(),
                    suspended: true,
                };
            }
        }

        self.flash
            .read_4kib(address, read_length, read_buffer)
            .await
    }

    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        if let EraseState::Erasing { range, suspended } = &self.erase_state {
            if range.contains(&address) {
                self.wait_prev_erase().await?;
            } else {
                if !*suspended {
                    self.flash.suspend_erase().await?;
                    self.erase_state = EraseState::Erasing {
                        range: range.clone(),
                        suspended: true,
                    };
                }

                // if *suspended {
                //     self.flash.resume_erase().await?;
                // }
                // self.timer.sleep(self.tune.erase_ms_every_write_256b).await;
                // if self.flash.erase_finished().await? {
                //     self.erase_state = EraseState::Idle;
                // } else {
                //     self.flash.suspend_erase().await?;
                //     self.erase_state = EraseState::Erasing {
                //         range: range.clone(),
                //         suspended: true,
                //     };
                // }
            }
        }

        let result = self.flash.write_256b(address, write_buffer).await;

        if let EraseState::Erasing { range, suspended:true } = &self.erase_state {
            self.flash.resume_erase().await?;
            self.erase_state = EraseState::Erasing {
                range: range.clone(),
                suspended: false,
            };
        }

        result
    }
}
