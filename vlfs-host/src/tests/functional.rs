use crate::{get_test_image_path, tests::harness::VLFSTestingHarness};
use function_name::named;
use log::LevelFilter;
use vlfs::{FileID, FileType};

fn init_logger() {
    let _ = env_logger::builder()
        .filter_level(LevelFilter::Error)
        .filter(Some("vlfs"), LevelFilter::Trace)
        .filter(Some("vlfs-host"), LevelFilter::Trace)
        .is_test(true)
        .try_init();
}

macro_rules! test_write_read {
    ($name:ident, $length:expr) => {
        #[named]
        #[tokio::test]
        async fn $name() {
            init_logger();

            let path = get_test_image_path!();

            let mut harness = VLFSTestingHarness::new(path).await;
            let file_id = harness.create_file(FileType(0)).await;
            harness.open_file_for_write(file_id).await;
            harness.append_file(file_id, $length).await.unwrap();
            harness.close_write_file(file_id).await;

            harness.reinit().await;

            harness.open_file_for_read(file_id).await;
            harness.read_file(file_id, $length).await;
            harness.close_read_file(file_id).await;
        }
    };
}

test_write_read!(write_read_0, 0);
test_write_read!(write_read_1, 1);
test_write_read!(write_read_2, 2);
test_write_read!(write_read_3, 3);
test_write_read!(write_read_4, 4);
test_write_read!(write_read_5, 5);
test_write_read!(write_read_251, 251);
test_write_read!(write_read_252, 252);
test_write_read!(write_read_253, 253);
test_write_read!(write_read_503, 503);
test_write_read!(write_read_504, 504);
test_write_read!(write_read_505, 505);
test_write_read!(write_read_3779, 3779);
test_write_read!(write_read_3780, 3780);
test_write_read!(write_read_3781, 3781);
test_write_read!(write_read_4015, 4015);
test_write_read!(write_read_4016, 4016);
test_write_read!(write_read_4017, 4017);
test_write_read!(write_read_8031, 8031);
test_write_read!(write_read_8032, 8032);
test_write_read!(write_read_8033, 8033);
test_write_read!(write_read_1m, 1024 * 1024);
test_write_read!(write_read_16m, 1024 * 1024 * 16);
test_write_read!(write_read_32m, 1024 * 1024 * 32);
test_write_read!(write_read_full_disk, 65669631);

#[named]
#[tokio::test]
async fn free_space_empty() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;
    assert_eq!(harness.get_free_space().await, 65669632);
}

#[named]
#[tokio::test]
async fn free_space_one_file() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;
    harness.append_file(file_id, 1).await.unwrap();
    harness.close_write_file(file_id).await;
    assert_eq!(harness.get_free_space().await, 65669632 - 4016);
}

#[named]
#[tokio::test]
async fn free_space_two_files() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;
    harness.append_file(file_id, 1).await.unwrap();
    harness.close_write_file(file_id).await;
    let file_id = harness.create_file(FileType(2)).await;
    harness.open_file_for_write(file_id).await;
    harness.append_file(file_id, 2).await.unwrap();
    harness.close_write_file(file_id).await;
    assert_eq!(harness.get_free_space().await, 65669632 - 4016 * 2);
}

#[named]
#[tokio::test]
async fn disk_full() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;
    harness.append_file(file_id, 65669632).await.unwrap_err();
    harness.close_write_file(file_id).await;
}

#[named]
#[tokio::test]
async fn open_file_twice() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    let writer1 = harness.vlfs.open_file_for_write(file_id).await.unwrap();
    let _ = harness.vlfs.open_file_for_write(file_id).await.unwrap_err();
    writer1.close().await.unwrap();
}

#[named]
#[tokio::test]
async fn open_file_doesnt_exist() {
    init_logger();

    let path = get_test_image_path!();

    let harness = VLFSTestingHarness::new(path).await;
    let _ = harness
        .vlfs
        .open_file_for_write(FileID(1))
        .await
        .unwrap_err();
}
