use crate::{get_test_image_path, tests::harness::VLFSTestingHarness};
use function_name::named;
use vlfs::{FileType, VLFSError};

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
#[named]
#[tokio::test]
async fn disk_full() {
    env_logger::init();

    let path = get_test_image_path!();
    // TODO: find max file size first
    let max_file_size = 65931264;

    // TODO: simulate a file with max size in memory and on disk and test with said vlfs instance
    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;
    harness.append_file(file_id, max_file_size).await;

    harness.reinit().await;

    harness.open_file_for_read(file_id).await;
    harness.read_file(file_id, max_file_size).await;
}

// Use Binary Search to find maximum siE of a file that can be written to disk
// This will be done by setting lower and upper bound (0MB and 64MB), then writing a file of size (lower + upper) / 2
//TODO: Check if it works :)
#[named]
#[tokio::test]
async fn find_max_file_using_harness() {
    let mut lower = 0;
    let mut upper = 64 * 1024 * 1024; // 64MB

    while lower <= upper {
        let mid = (lower + upper) / 2;
    
        println!(">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>><<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<");
        println!(
            ">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>{}<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<",
            mid
        );
        println!(">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>><<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<");

        let path = get_test_image_path!();
        let mut harness = VLFSTestingHarness::new(path).await;
        let file_id = harness.create_file(FileType(0)).await;

        harness.open_file_for_write(file_id).await;
        let result = harness.append_file_with_result(file_id, mid).await;

        println!("Result is {:?}", result);

        // Check if result is Err(DeviceFull)
        if result.is_err() {
            upper = mid - 1;
        } else {
            lower = mid + 1;
        }
    }

    println!("Max file size is {}", lower);
}

//TODO: Write a test for opening file twice
// #[named]
// #[tokio::test]
// async fn open_file_twice() {
//     env_logger::init();

//     let path = get_test_image_path!();

//     let mut harness = VLFSTestingHarness::new(path).await;
//     let file_id = harness.create_file(FileType(0)).await;
//     harness.open_file_for_write(file_id).await;

//     let result = harness.vlfs.open_file_for_write(file_id).await;
//     assert!(result.is_err());
// }