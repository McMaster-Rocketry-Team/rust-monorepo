pub trait SysReset: Copy {
    fn reset(self) -> !;
}

#[derive(Copy, Clone)]
pub struct PanicSysReset {}

impl SysReset for PanicSysReset {
    fn reset(self) -> ! {
        defmt::panic!("Reset requested")
    }
}
