pub trait SysReset: Copy {
    fn reset(self) -> !;
}
