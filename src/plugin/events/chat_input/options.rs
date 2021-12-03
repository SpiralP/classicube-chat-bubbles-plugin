use classicube_sys::{
    keybindNames, InputButtons, InputButtons_INPUT_COUNT, InputButtons_KEY_ESCAPE, Input_Names,
    KeyBind, KeyBind_Defaults, Options_GetEnum,
};
use std::{ffi::CString, os::raw::c_int};

pub fn get_input_button(key: KeyBind) -> Option<InputButtons> {
    let name = CString::new(format!("key-{}", keybindNames[key as usize])).unwrap();

    let input_names = Input_Names
        .iter()
        .map(|&name| CString::new(name).unwrap())
        .collect::<Vec<_>>();
    let input_names = input_names
        .iter()
        .map(|c_string| c_string.as_ptr())
        .collect::<Vec<_>>();

    let mapping = unsafe {
        Options_GetEnum(
            name.as_ptr(),
            KeyBind_Defaults[key as usize] as c_int,
            input_names.as_ptr(),
            InputButtons_INPUT_COUNT,
        )
    };
    if mapping != InputButtons_KEY_ESCAPE {
        Some(mapping)
    } else {
        None
    }
}
