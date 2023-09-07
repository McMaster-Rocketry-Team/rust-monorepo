use crate::{get_test_image_path, tests::harness::VLFSTestingHarness};
use function_name::named;
use vlfs::FileType;

#[named]
#[tokio::test]
async fn write_read() {
    env_logger::init();
    
    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;
    harness.append_file(file_id, 1000).await;

    harness.reinit().await;

    harness.open_file_for_read(file_id).await;
    harness.read_file(file_id, 1000).await;
}

// test for disk full
// test for opening a file twice (should not work)
// test for opening a file that doesn't exist
