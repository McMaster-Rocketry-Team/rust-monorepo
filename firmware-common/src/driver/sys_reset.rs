pub trait SysReset: Copy {
    fn reset(&self) -> !;
}

#[derive(Copy, Clone)]
pub struct PanicSysReset {}

impl SysReset for PanicSysReset {
    fn reset(&self) -> ! {
        log_panic!("Reset requested")
    }
}
