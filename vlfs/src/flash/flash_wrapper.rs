use core::ops::{Deref, DerefMut};

use embassy_sync::{
    blocking_mutex::raw::{NoopRawMutex, RawMutex},
    mutex::Mutex,
};

use crate::{utils::rwlock::RwLock, Flash};

pub struct FlashWrapper<F: Flash> {
    flash: Mutex<NoopRawMutex, F>,
}

impl<F: Flash> FlashWrapper<F> {
    pub fn new(flash: F) -> Self {
        Self {
            flash: Mutex::new(flash),
        }
    }

    pub async fn size(&self) -> u32 {
        let flash = self.flash.lock().await;
        flash.size().await
    }

    pub async fn reset(&mut self) -> Result<(), F::Error> {
        let mut flash = self.flash.lock().await;
        flash.reset().await
    }

    pub async fn erase_sector_4kib(&mut self, sector: u32) -> Result<(), F::Error> {
        let mut flash = self.flash.lock().await;
        flash.erase_sector_4kib(sector).await
    }

    pub async fn erase_block_32kib(&mut self, block: u32) -> Result<(), F::Error> {
        let mut flash = self.flash.lock().await;
        flash.erase_block_32kib(block).await
    }

    pub async fn erase_block_64kib(&mut self, block: u32) -> Result<(), F::Error> {
        let mut flash = self.flash.lock().await;
        flash.erase_block_64kib(block).await
    }

    pub async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), F::Error> {
        let mut flash = self.flash.lock().await;
        flash.write_256b(address, write_buffer).await
    }

    pub async fn write<'b>(
        &mut self,
        address: u32,
        write_length: usize,
        write_buffer: &'b mut [u8],
    ) -> Result<(), F::Error> {
        let mut flash = self.flash.lock().await;
        flash.write(address, write_length, write_buffer).await
    }

    pub async fn read_4kib<'b>(
        &self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], F::Error> {
        let mut flash = self.flash.lock().await;
        flash.read_4kib(address, read_length, read_buffer).await
    }

    pub async fn read<'b>(
        &self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], F::Error> {
        let mut flash = self.flash.lock().await;
        flash.read(address, read_length, read_buffer).await
    }

    pub fn into_inner(self) -> F {
        self.flash.into_inner()
    }
}

pub trait WritableFlash {
    type Error;

    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error>;

    async fn write<'b>(
        &mut self,
        address: u32,
        write_length: usize,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error>;
}

pub trait ReadableFlash {
    type Error;

    async fn read_4kib<'b>(
        &self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error>;

    async fn read<'b>(
        &self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error>;
}

impl<F: Flash> WritableFlash for FlashWrapper<F> {
    type Error = F::Error;

    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        self.write_256b(address, write_buffer).await
    }

    async fn write<'b>(
        &mut self,
        address: u32,
        write_length: usize,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        self.write(address, write_length, write_buffer).await
    }
}

impl<F: Flash, T: DerefMut<Target = FlashWrapper<F>>> WritableFlash for T {
    type Error = F::Error;

    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        self.deref_mut().write_256b(address, write_buffer).await
    }

    async fn write<'b>(
        &mut self,
        address: u32,
        write_length: usize,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        self.deref_mut()
            .write(address, write_length, write_buffer)
            .await
    }
}

impl<M: RawMutex, F: Flash, const N: usize> WritableFlash for &RwLock<M, FlashWrapper<F>, N> {
    type Error = F::Error;

    async fn write_256b<'b>(
        &mut self,
        address: u32,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        RwLock::write(&self).await.write_256b(address, write_buffer).await
    }

    async fn write<'b>(
        &mut self,
        address: u32,
        write_length: usize,
        write_buffer: &'b mut [u8],
    ) -> Result<(), Self::Error> {
        RwLock::write(&self).await.write(address, write_length, write_buffer).await
    }
}

impl<F: Flash> ReadableFlash for FlashWrapper<F> {
    type Error = F::Error;

    async fn read_4kib<'b>(
        &self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        self.read_4kib(address, read_length, read_buffer).await
    }

    async fn read<'b>(
        &self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        self.read(address, read_length, read_buffer).await
    }
}

impl<F: Flash, T: Deref<Target = FlashWrapper<F>>> ReadableFlash for T {
    type Error = F::Error;

    async fn read_4kib<'b>(
        &self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        self.deref()
            .read_4kib(address, read_length, read_buffer)
            .await
    }

    async fn read<'b>(
        &self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        self.deref().read(address, read_length, read_buffer).await
    }
}

impl<M: RawMutex, F: Flash, const N: usize> ReadableFlash for &RwLock<M, FlashWrapper<F>, N> {
    type Error = F::Error;
    
    async fn read_4kib<'b>(
        &self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        RwLock::read(&self).await.read_4kib(address, read_length, read_buffer).await
    }
    
    async fn read<'b>(
        &self,
        address: u32,
        read_length: usize,
        read_buffer: &'b mut [u8],
    ) -> Result<&'b [u8], Self::Error> {
        RwLock::read(&self).await.read(address, read_length, read_buffer).await
    }

}