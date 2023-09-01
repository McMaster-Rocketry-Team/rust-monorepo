#[macro_export]
macro_rules! get_test_image_path {
    () => {{
        use project_root::get_project_root;
        let mut path = get_project_root().unwrap();
        path.push("vlfs-host");
        path.push("test-images");
        path.push(format!("{}.vlfs", function_name!()));

        if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }
        
        path
    }};
}
