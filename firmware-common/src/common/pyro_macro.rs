#[macro_export]
macro_rules! pyro {
    ($device_manager:ident, $pyro_selection:ident$(.$pyro_selection_field:ident)*, pyro_cont.$($call:tt)*) =>{
        match $pyro_selection$(.$pyro_selection_field)* {
            PyroSelection::Pyro1 => {
                claim_devices!($device_manager, pyro1_cont);
                pyro1_cont.$($call)*
            },
            PyroSelection::Pyro2 => {
                claim_devices!($device_manager, pyro2_cont);
                pyro2_cont.$($call)*
            },
            PyroSelection::Pyro3 => {
                claim_devices!($device_manager, pyro2_cont);
                pyro2_cont.$($call)*
            },
        }
    };
    ($device_manager:ident, $pyro_selection:ident$(.$pyro_selection_field:ident)*, pyro_ctrl.$($call:tt)*) =>{
        match $pyro_selection$(.$pyro_selection_field)* {
            PyroSelection::Pyro1 => {
                claim_devices!($device_manager, pyro1_ctrl);
                pyro1_ctrl.$($call)*
            },
            PyroSelection::Pyro2 => {
                claim_devices!($device_manager, pyro2_ctrl);
                pyro2_ctrl.$($call)*
            },
            PyroSelection::Pyro3 => {
                claim_devices!($device_manager, pyro3_ctrl);
                pyro3_ctrl.$($call)*
            },
        }
    };
}
