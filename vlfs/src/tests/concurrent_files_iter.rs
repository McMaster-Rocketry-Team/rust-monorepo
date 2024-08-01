use crate::tests::init_logger;
use crate::{get_test_image_path, tests::harness::VLFSTestingHarness};
use crate::{FileEntry, FileID, FileType};
use function_name::named;

#[named]
#[tokio::test]
async fn concurrent_files_iter_no_changes() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    for i in 0..5 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness.append_file(file_id, i as usize).await.unwrap();
        harness.close_write_file(file_id).await;
    }

    let mut files = Vec::<FileEntry>::new();
    let mut iter = harness.vlfs.concurrent_files_iter(()).await;
    while let Some(file) = iter.next().await.unwrap() {
        files.push(file);
    }
    assert_eq!(
        files
            .iter()
            .map(|file_entry| file_entry.id)
            .collect::<Vec<_>>(),
        vec![FileID(1), FileID(2), FileID(3), FileID(4), FileID(5)]
    );
}

#[named]
#[tokio::test]
async fn concurrent_files_iter_no_changes_filter() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    for i in 0..5 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness.append_file(file_id, i as usize).await.unwrap();
        harness.close_write_file(file_id).await;
    }

    let mut files = Vec::<FileEntry>::new();
    let mut iter = harness.vlfs.concurrent_files_iter(|entry:&FileEntry| entry.id.0 % 2 == 0).await;
    while let Some(file) = iter.next().await.unwrap() {
        files.push(file);
    }
    assert_eq!(
        files
            .iter()
            .map(|file_entry| file_entry.id)
            .collect::<Vec<_>>(),
        vec![FileID(2), FileID(4)]
    );
}


#[named]
#[tokio::test]
async fn concurrent_files_iter_delete_1() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    for i in 0..5 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness.append_file(file_id, i as usize).await.unwrap();
        harness.close_write_file(file_id).await;
    }

    let mut files = Vec::<FileEntry>::new();
    let mut iter = harness.vlfs.concurrent_files_iter(()).await;
    for _i in 0..3 {
        files.push(iter.next().await.unwrap().unwrap());
    }
    harness.remove_file(FileID(1)).await;
    for _i in 0..2 {
        files.push(iter.next().await.unwrap().unwrap());
    }
    assert_eq!(iter.next().await.unwrap(), None);
    assert_eq!(
        files
            .iter()
            .map(|file_entry| file_entry.id)
            .collect::<Vec<_>>(),
        vec![FileID(1), FileID(2), FileID(3), FileID(4), FileID(5)]
    );
}

#[named]
#[tokio::test]
async fn concurrent_files_iter_delete_1_filter() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    for i in 0..5 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness.append_file(file_id, i as usize).await.unwrap();
        harness.close_write_file(file_id).await;
    }

    let mut iter = harness.vlfs.concurrent_files_iter(|entry:&FileEntry| entry.id.0 % 2 == 0).await;
    assert_eq!(iter.next().await.unwrap().unwrap().id, FileID(2));
    harness.remove_file(FileID(4)).await;
    assert_eq!(iter.next().await.unwrap(), None);
}

#[named]
#[tokio::test]
async fn concurrent_files_iter_delete_last_iterated() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    for i in 0..5 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness.append_file(file_id, i as usize).await.unwrap();
        harness.close_write_file(file_id).await;
    }

    let mut files = Vec::<FileEntry>::new();
    let mut iter = harness.vlfs.concurrent_files_iter(()).await;
    for _i in 0..3 {
        files.push(iter.next().await.unwrap().unwrap());
    }
    harness.remove_file(FileID(3)).await;
    for _i in 0..2 {
        files.push(iter.next().await.unwrap().unwrap());
    }
    assert_eq!(iter.next().await.unwrap(), None);
    assert_eq!(
        files
            .iter()
            .map(|file_entry| file_entry.id)
            .collect::<Vec<_>>(),
        vec![FileID(1), FileID(2), FileID(3), FileID(4), FileID(5)]
    );
}

#[named]
#[tokio::test]
async fn concurrent_files_iter_delete_2() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    for i in 0..5 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness.append_file(file_id, i as usize).await.unwrap();
        harness.close_write_file(file_id).await;
    }

    let mut files = Vec::<FileEntry>::new();
    let mut iter = harness.vlfs.concurrent_files_iter(()).await;
    for _i in 0..3 {
        files.push(iter.next().await.unwrap().unwrap());
    }
    harness.remove_file(FileID(1)).await;
    harness.remove_file(FileID(2)).await;
    for _i in 0..2 {
        files.push(iter.next().await.unwrap().unwrap());
    }
    assert_eq!(iter.next().await.unwrap(), None);
    assert_eq!(
        files
            .iter()
            .map(|file_entry| file_entry.id)
            .collect::<Vec<_>>(),
        vec![FileID(1), FileID(2), FileID(3), FileID(4), FileID(5)]
    );
}

#[named]
#[tokio::test]
async fn concurrent_files_iter_delete_all_before() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    for i in 0..5 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness.append_file(file_id, i as usize).await.unwrap();
        harness.close_write_file(file_id).await;
    }

    let mut files = Vec::<FileEntry>::new();
    let mut iter = harness.vlfs.concurrent_files_iter(()).await;
    for _i in 0..3 {
        files.push(iter.next().await.unwrap().unwrap());
    }
    harness.remove_file(FileID(1)).await;
    harness.remove_file(FileID(2)).await;
    harness.remove_file(FileID(3)).await;
    for _i in 0..2 {
        files.push(iter.next().await.unwrap().unwrap());
    }
    assert_eq!(iter.next().await.unwrap(), None);
    assert_eq!(
        files
            .iter()
            .map(|file_entry| file_entry.id)
            .collect::<Vec<_>>(),
        vec![FileID(1), FileID(2), FileID(3), FileID(4), FileID(5)]
    );
}

#[named]
#[tokio::test]
async fn concurrent_files_iter_delete_all_then_create() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    for i in 0..5 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness.append_file(file_id, i as usize).await.unwrap();
        harness.close_write_file(file_id).await;
    }

    let mut files = Vec::<FileEntry>::new();
    let mut iter = harness.vlfs.concurrent_files_iter(()).await;
    for _i in 0..3 {
        files.push(iter.next().await.unwrap().unwrap());
    }
    harness.remove_file(FileID(1)).await;
    harness.remove_file(FileID(2)).await;
    harness.remove_file(FileID(3)).await;
    harness.remove_file(FileID(4)).await;
    harness.remove_file(FileID(5)).await;
    assert_eq!(iter.next().await.unwrap(), None);
    assert_eq!(
        files
            .iter()
            .map(|file_entry| file_entry.id)
            .collect::<Vec<_>>(),
        vec![FileID(1), FileID(2), FileID(3)]
    );

    harness.create_file(FileType(0)).await;
    harness.create_file(FileType(0)).await;
    assert_eq!(iter.next().await.unwrap().unwrap().id, FileID(6));
    assert_eq!(iter.next().await.unwrap().unwrap().id, FileID(7));
    assert_eq!(iter.next().await.unwrap(), None);

    harness.verify_invariants().await;
}

#[named]
#[tokio::test]
async fn concurrent_files_iter_delete_some_then_create() {
    init_logger();

    let path = get_test_image_path!();

    let mut harness = VLFSTestingHarness::new(path).await;

    for i in 0..5 {
        let file_id = harness.create_file(FileType(i)).await;
        harness.open_file_for_write(file_id).await;
        harness.append_file(file_id, i as usize).await.unwrap();
        harness.close_write_file(file_id).await;
    }

    let mut iter = harness.vlfs.concurrent_files_iter(()).await;
    assert_eq!(iter.next().await.unwrap().unwrap().id, FileID(1));
    assert_eq!(iter.next().await.unwrap().unwrap().id, FileID(2));
    assert_eq!(iter.next().await.unwrap().unwrap().id, FileID(3));
    harness.remove_file(FileID(2)).await;
    harness.remove_file(FileID(3)).await;
    harness.remove_file(FileID(4)).await;
    harness.create_file(FileType(0)).await;
    assert_eq!(iter.next().await.unwrap().unwrap().id, FileID(5));
    assert_eq!(iter.next().await.unwrap().unwrap().id, FileID(6));
    assert_eq!(iter.next().await.unwrap(), None);

    let mut iter = harness.vlfs.concurrent_files_iter(()).await;
    assert_eq!(iter.next().await.unwrap().unwrap().id, FileID(1));
    assert_eq!(iter.next().await.unwrap().unwrap().id, FileID(5));
    assert_eq!(iter.next().await.unwrap().unwrap().id, FileID(6));
    assert_eq!(iter.next().await.unwrap(), None);

    harness.verify_invariants().await;
}
