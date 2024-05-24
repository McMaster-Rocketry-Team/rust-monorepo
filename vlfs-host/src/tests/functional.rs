use crate::{get_test_image_path, tests::harness::VLFSTestingHarness};
use function_name::named;
use vlfs::FileType;

fn init_logger() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::max())
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
            harness.append_file(file_id, $length).await;
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

// test for disk full
// test for opening a file twice (should not work)
// test for opening a file that doesn't exist
