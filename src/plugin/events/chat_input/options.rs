use classicube_sys::{
    bindNames, InputBind, InputButtons, InputButtons_CCKEY_ESCAPE, InputButtons_INPUT_COUNT,
    Input_StorageNames, KeyBind_Defaults, Options_GetEnum,
};
use std::{ffi::CString, os::raw::c_int};

pub fn get_input_button(key: InputBind) -> Option<InputButtons> {
    let name = CString::new(format!("key-{}", bindNames[key as usize])).unwrap();

    let input_names = Input_StorageNames
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
            KeyBind_Defaults[key as usize].button1 as c_int,
            input_names.as_ptr(),
            InputButtons_INPUT_COUNT as _,
        ) as InputButtons
    };
    if mapping != InputButtons_CCKEY_ESCAPE {
        Some(mapping)
    } else {
        None
    }
}
