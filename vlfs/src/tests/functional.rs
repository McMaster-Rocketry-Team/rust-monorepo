use crate::tests::init_logger;
use crate::{get_test_image_path, tests::harness::VLFSTestingHarness};
use crate::{FileEntry, FileID, FileType};
use function_name::named;

#[named]
#[tokio::test]
async fn write_read_empty() {
    init_logger();
    let path = get_test_image_path!();
    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;
    harness.close_write_file(file_id).await;
    harness.reinit().await;
    harness.open_file_for_read(file_id).await;
    harness.read_file(file_id, 0).await;
    harness.close_read_file(file_id).await;
    harness.verify_invariants().await;
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

            harness.verify_invariants().await;
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
// skip the following tests when running coverage because they take too long
#[cfg(not(feature = "internal_test_coverage"))]
test_write_read!(write_read_16m, 1024 * 1024 * 16);
#[cfg(not(feature = "internal_test_coverage"))]
test_write_read!(write_read_32m, 1024 * 1024 * 32);
test_write_read!(write_read_full_disk, 65669631);

#[named]
#[tokio::test]
async fn write_read_three_files() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id1 = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id1).await;
    harness.append_file(file_id1, 1).await.unwrap();
    harness.close_write_file(file_id1).await;
    let file_id2 = harness.create_file(FileType(1)).await;
    let file_id3 = harness.create_file(FileType(2)).await;
    harness.open_file_for_write(file_id3).await;
    harness.append_file(file_id3, 3).await.unwrap();
    harness.close_write_file(file_id3).await;

    harness.open_file_for_write(file_id2).await;
    harness.append_file(file_id2, 2).await.unwrap();
    harness.close_write_file(file_id2).await;

    harness.verify_invariants().await;
}

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
    harness.verify_invariants().await;
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

#[named]
#[tokio::test]
async fn write_read_10_files() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;
    let mut file_ids = Vec::<FileID>::new();

    for i in 0..10 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness
            .append_file(file_id, i as usize * 10000)
            .await
            .unwrap();
        harness.close_write_file(file_id).await;
        file_ids.push(file_id);
    }

    for i in 0..10 {
        let file_id = file_ids[i];
        harness.open_file_for_read(file_id).await;
        harness.read_file(file_id, i as usize * 10000).await;
        harness.close_read_file(file_id).await;
    }

    harness.reinit().await;

    for i in 0..10 {
        let file_id = file_ids[i];
        harness.open_file_for_read(file_id).await;
        harness.read_file(file_id, i as usize * 10000).await;
        harness.close_read_file(file_id).await;
    }
}

#[named]
#[tokio::test]
async fn iterate_files() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    for i in 0..10 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness.append_file(file_id, i as usize).await.unwrap();
        harness.close_write_file(file_id).await;
    }

    harness.verify_invariants().await;

    harness.reinit().await;

    harness.verify_invariants().await;
}

#[named]
#[tokio::test]
async fn write_read_file_with_flush() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;

    for i in 0..10 {
        harness
            .append_file(file_id, i as usize * 10000)
            .await
            .unwrap();
        harness.flush_file(file_id).await;
    }

    harness.close_write_file(file_id).await;

    harness.open_file_for_read(file_id).await;
    harness.read_file(file_id, 450000).await;
    harness.close_read_file(file_id).await;
    harness.verify_invariants().await;

    harness.reinit().await;
    harness.open_file_for_read(file_id).await;
    harness.read_file(file_id, 450000).await;
    harness.close_read_file(file_id).await;
    harness.verify_invariants().await;
}

#[named]
#[tokio::test]
async fn remove_file_a() {
    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;
    harness.append_file(file_id, 1).await.unwrap();
    harness.close_write_file(file_id).await;

    harness.remove_file(file_id).await;
    harness.verify_invariants().await;
}

#[named]
#[tokio::test]
async fn remove_file_a2() {
    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;
    harness.append_file(file_id, 1).await.unwrap();
    harness.close_write_file(file_id).await;

    harness.remove_file(file_id).await;
    harness.reinit().await;
    harness.verify_invariants().await;
}

#[named]
#[tokio::test]
async fn remove_file_b() {
    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;
    harness.append_file(file_id, 1).await.unwrap();
    harness.close_write_file(file_id).await;

    let file_id2 = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id2).await;
    harness.append_file(file_id2, 10000).await.unwrap();
    harness.close_write_file(file_id2).await;

    harness.remove_file(file_id).await;
    harness.verify_invariants().await;
}

#[named]
#[tokio::test]
async fn remove_opened_file() {
    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = harness.create_file(FileType(0)).await;
    harness.open_file_for_write(file_id).await;
    harness.append_file(file_id, 1).await.unwrap();

    harness.remove_file(file_id).await;
    harness.verify_invariants().await;
}

#[named]
#[tokio::test]
async fn remove_non_existent_file() {
    init_logger();

    let path = get_test_image_path!();
    let mut harness = VLFSTestingHarness::new(path).await;
    let file_id = FileID(100);

    harness.remove_file(file_id).await;
    harness.verify_invariants().await;
}

#[named]
#[tokio::test]
async fn remove_files() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    for i in 0..10 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness.append_file(file_id, i as usize).await.unwrap();
        harness.close_write_file(file_id).await;
    }

    harness.remove_files(|id| id.0 % 2 == 0).await;
    harness.verify_invariants().await;
}

#[named]
#[tokio::test]
async fn files_iter() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    for i in 0..10 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness.append_file(file_id, i as usize).await.unwrap();
        harness.close_write_file(file_id).await;
    }

    harness.verify_invariants().await;
}

#[named]
#[tokio::test]
async fn files_iter_filter() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    for i in 0..10 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness.append_file(file_id, i as usize).await.unwrap();
        harness.close_write_file(file_id).await;
    }

    harness.verify_invariants().await;

    let mut files = Vec::<FileEntry>::new();
    let mut iter = harness
        .vlfs
        .files_iter_filter(|entry| entry.id.0 % 2 == 0)
        .await;
    while let Some(file) = iter.next().await.unwrap() {
        files.push(file);
    }

    assert_eq!(
        files
            .iter()
            .map(|file_entry| file_entry.id)
            .collect::<Vec<_>>(),
        vec![FileID(2), FileID(4), FileID(6), FileID(8), FileID(10)]
    );
}

#[named]
#[tokio::test]
async fn files_iter_no_file() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    harness.verify_invariants().await;
}
