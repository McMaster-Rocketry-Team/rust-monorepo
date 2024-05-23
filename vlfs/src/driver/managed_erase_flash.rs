use core::ops::Range;

use embedded_hal_async::delay::DelayNs;

use crate::Flash;

use super::async_erase_flash::AsyncEraseFlash;

pub struct EraseTune {
    pub erase_us_every_write_256b: u32,
}

enum EraseState {
    Idle,
    Erasing { range: Range<u32>, suspended: bool },
}

pub struct ManagedEraseFlash<AF: AsyncEraseFlash, D: DelayNs> {
    flash: AF,
    erase_state: EraseState,
    tune: EraseTune,
    delay: D,
}

impl<AF: AsyncEraseFlash, D: DelayNs> ManagedEraseFlash<AF, D> {
    pub fn new(flash: AF, delay: D, tune: EraseTune) -> Self {
        Self {
            flash,
            erase_state: EraseState::Idle,
            tune,
            delay,
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

    async fn suspend_erase(&mut self) -> Result<(), AF::Error> {
        if let EraseState::Erasing {
            ref range,
            suspended: false,
        } = self.erase_state
        {
            self.flash.suspend_erase().await?;
            self.erase_state = EraseState::Erasing {
                range: range.clone(),
                suspended: true,
            };
        }
        Ok(())
    }

    async fn resume_erase(&mut self) -> Result<(), AF::Error> {
        if let EraseState::Erasing {
            ref range,
            suspended: true,
        } = self.erase_state
        {
            self.flash.resume_erase().await?;
            self.erase_state = EraseState::Erasing {
                range: range.clone(),
                suspended: false,
            };
        }
        Ok(())
    }
}

impl<AF: AsyncEraseFlash, D: DelayNs> Flash for ManagedEraseFlash<AF, D> {
    type Error = AF::Error;

    async fn size(&self) -> u32 {
        self.flash.size().await
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
                self.suspend_erase().await?;
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
        if let EraseState::Erasing {
            range,
            suspended: _,
        } = &self.erase_state
        {
            if range.contains(&address) {
                self.wait_prev_erase().await?;
            } else {
                self.suspend_erase().await?;
            }
        }

        let result = self.flash.write_256b(address, write_buffer).await;

        if let EraseState::Erasing {
            range: _,
            suspended: true,
        } = &self.erase_state
        {
            self.resume_erase().await?;
            self.delay
                .delay_us(self.tune.erase_us_every_write_256b)
                .await;

            if !self.flash.is_busy().await? {
                self.erase_state = EraseState::Idle;
            }
        }

        result
    }
}
