pub trait DeviceManagement: Copy {
    fn reset(self) -> !;
}
