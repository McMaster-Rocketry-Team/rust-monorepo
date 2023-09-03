use std::{collections::HashMap, mem::transmute, path::PathBuf, pin::Pin};

use futures::executor::block_on;
use rand::Rng;
use replace_with::replace_with_or_abort;
use vlfs::{
    io_traits::{AsyncReader, AsyncWriter},
    DummyCrc, FileID, FileReader, FileType, FileWriter, VLFS,
};

use crate::FileFlash;

pub struct VLFSTestingHarness {
    flash_image_path: PathBuf,
    pub vlfs: Pin<Box<VLFS<FileFlash, DummyCrc>>>,
    pub files: HashMap<FileID, (FileType, Vec<u8>)>,
    pub file_writers: HashMap<FileID, FileWriter<'static, FileFlash, DummyCrc>>,
    pub file_readers: HashMap<FileID, (FileReader<'static, FileFlash, DummyCrc>, usize)>, // cursor position
}

impl VLFSTestingHarness {
    pub async fn new(flash_image_path: PathBuf) -> Self {
        let flash = FileFlash::new(flash_image_path.clone()).await.unwrap();
        let mut vlfs = VLFS::new(flash, DummyCrc {});
        vlfs.init().await.unwrap();
        Self {
            flash_image_path,
            vlfs: Box::pin(vlfs),
            files: HashMap::new(),
            file_writers: HashMap::new(),
            file_readers: HashMap::new(),
        }
    }

    pub async fn reinit(&mut self) {
        replace_with_or_abort(&mut self.vlfs, |old_fs| {
            for (_, file_writer) in self.file_writers.drain() {
                block_on(file_writer.close()).unwrap();
            }
            for (_, (file_reader, _)) in self.file_readers.drain() {
                block_on(file_reader.close());
            }
            drop(old_fs);
            let flash_fut = FileFlash::new(self.flash_image_path.clone());
            let flash = block_on(flash_fut).unwrap();
            Box::pin(VLFS::new(flash, DummyCrc {}))
        });
        self.vlfs.init().await.unwrap();
    }

    pub async fn create_file(&mut self, file_type: FileType) -> FileID {
        let file_entry = self.vlfs.create_file(file_type).await.unwrap();
        self.files.insert(file_entry.id, (file_type, Vec::new()));
        file_entry.id
    }

    pub async fn open_file_for_write(&mut self, file_id: FileID) {
        let file_writer = self.vlfs.open_file_for_write(file_id).await.unwrap();
        self.file_writers.insert(file_id, unsafe {
            // SAFETY: The reference inside the Pin<Box<VLFS>> is guaranteed not to move out of its heap allocation
            transmute(file_writer)
        });
    }

    pub async fn append_file(&mut self, file_id: FileID, length: usize) {
        let file_writer = self.file_writers.get_mut(&file_id).unwrap();

        let mut rng = rand::thread_rng();
        let mut vec = Vec::with_capacity(length);
        for _ in 0..length {
            vec.push(rng.gen::<u8>());
        }

        self.files
            .get_mut(&file_id)
            .unwrap()
            .1
            .extend_from_slice(vec.as_slice());
        file_writer.extend_from_slice(vec.as_slice()).await.unwrap();
    }

    pub async fn flush_file(&mut self, file_id: FileID) {
        let file_writer = self.file_writers.get_mut(&file_id).unwrap();
        file_writer.flush().await.unwrap();
    }

    pub async fn close_write_file(&mut self, file_id: FileID) {
        let file_writer = self.file_writers.remove(&file_id).unwrap();
        file_writer.close().await.unwrap();
    }

    pub async fn open_file_for_read(&mut self, file_id: FileID) {
        let file_reader = self.vlfs.open_file_for_read(file_id).await.unwrap();
        self.file_readers.insert(file_id, unsafe {
            // SAFETY: The reference inside the Pin<Box<VLFS>> is guaranteed not to move out of its heap allocation
            (transmute(file_reader), 0)
        });
    }

    pub async fn read_file(&mut self, file_id: FileID, length: usize) {
        let (file_reader, cursor) = self.file_readers.get_mut(&file_id).unwrap();

        let mut buffer = vec![0u8; length];
        let (buffer, _) = file_reader
            .read_slice(buffer.as_mut_slice(), length)
            .await
            .unwrap();

        let expected_buffer = self.files.get(&file_id).unwrap().1.as_slice();
        assert_eq!(buffer, &expected_buffer[*cursor..*cursor + buffer.len()]);
        *cursor += buffer.len();

        if buffer.len() < length {
            assert_eq!(*cursor, expected_buffer.len());
        } else if buffer.len() > length {
            panic!("buffer.len() > length");
        }
    }

    pub async fn close_read_file(&mut self, file_id: FileID) {
        let (file_reader, _) = self.file_readers.remove(&file_id).unwrap();
        file_reader.close().await;
    }

    pub async fn validate_free_space(&mut self){
        todo!()
    }
}

impl Drop for VLFSTestingHarness {
    fn drop(&mut self) {
        for (_, file_writer) in self.file_writers.drain() {
            block_on(file_writer.close()).unwrap();
        }
        for (_, (file_reader, _)) in self.file_readers.drain() {
            block_on(file_reader.close());
        }
    }
}