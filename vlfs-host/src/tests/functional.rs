use crate::{get_test_image_path, tests::harness::VLFSTestingHarness};
use function_name::named;
use tokio::time::Instant;
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
#[named]
#[tokio::test]
async fn disk_full() {
    env_logger::init();

    let path = get_test_image_path!();
    let max_file_size = 65669632;

    // TODO: simulate a file with max size in memory and on disk and test with said vlfs instance
    // TODO: find max file size first
    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;
    harness.append_file(file_id, max_file_size).await;

    harness.reinit().await;

    harness.open_file_for_read(file_id).await;
    harness.read_file(file_id, max_file_size).await;
}

//TODO: Write a test for opening file twice
#[named]
#[tokio::test]
async fn open_file_twice() {
    env_logger::init();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;

    let result = harness.vlfs.open_file_for_write(file_id).await;
    assert!(result.is_err());
}


// Use Binary Search to find maximum siE of a file that can be written to disk
// This will be done by setting lower and upper bound (0MB and 64MB), then writing a file of size (lower + upper) / 2
//TODO: Check if it works :)
#[named]
#[tokio::test]
async fn find_max_file() {
    let mut lower = 0;
    let mut upper = 64 * 1024 * 1024; //64MB

    while lower<= upper {
        let mid = (lower + upper) / 2;

        let path = get_test_image_path!();
        let mut harness = VLFSTestingHarness::new(path).await;
        let file_id = harness.create_file(FileType(0)).await;
        harness.open_file_for_write(file_id).await;
        let result = harness.append_file_with_result(file_id, mid).await;

        if result.is_ok() {
            lower = mid;
        } else {
            upper = mid;
        }
    }

    println!("Max file size is {}", lower);
}

//TODO: Check if it works :)
// test if there is an issue with vlfs or my binary search
#[named]
#[tokio::test]
async fn test_harness_size(){
    let size = 6;
    let error_result;

    let path = get_test_image_path!();
    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;
    // append data of size "size" and return the result of the process
    let result = harness.append_file_with_result(file_id, size).await;

    if result.is_ok() {
        error_result = "No error"
    } else {
        error_result = "Error";

        // print the error
        println!("Error is {:?}", result);
    }

    println!("Result is {}", error_result);
}
