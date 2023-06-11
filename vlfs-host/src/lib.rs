#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
#![feature(let_chains)]
#![feature(try_blocks)]

pub use file_flash::FileFlash;

mod file_flash;

#[cfg(test)]
mod tests {
    use super::*;
    use vlfs::{DummyCrc, VLFS, FileType, io_traits::{AsyncWriter, AsyncReader}};

    #[tokio::test]
    async fn it_works() {
        let path = tempfile::Builder::new()
            .tempfile()
            .unwrap()
            .into_temp_path()
            .to_path_buf();
        let flash = FileFlash::new(path).await.unwrap();
        let mut vlfs = VLFS::new(flash, DummyCrc {});
        vlfs.init().await.unwrap();

        let file_id = vlfs.create_file(FileType(0)).await.unwrap();
        let mut file = vlfs.open_file_for_write(file_id).await.unwrap();
        file.extend_from_slice(b"Hello, world!").await.unwrap();
        file.close().await.unwrap();

        let mut file = vlfs.open_file_for_read(file_id).await.unwrap();
        let mut buffer = [0u8; 32];
        let (buffer, _) = file.read_all(&mut buffer).await.unwrap();
        file.close().await;

        assert_eq!(buffer, b"Hello, world!");
    }
}
