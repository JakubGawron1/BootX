//! Copyright (c) ChefKiss Inc 2021-2022.
//! This project is licensed by the Creative Commons Attribution-NoCommercial-NoDerivatives license.

use alloc::{vec, vec::Vec};

use uefi::{
    prelude::*,
    proto::media::file::{Directory, File, FileAttribute, FileInfo, FileMode, FileType},
    CStr16,
};

pub fn open_esp(image: Handle) -> Directory {
    unsafe {
        let fs = uefi_services::system_table()
            .as_mut()
            .boot_services()
            .get_image_file_system(image)
            .expect("Failed to get ESP")
            .interface
            .get()
            .as_mut()
            .unwrap();

        fs.open_volume().expect("Failed to open volume.")
    }
}

pub fn load(
    esp: &mut Directory,
    path: &CStr16,
    mode: FileMode,
    attributes: FileAttribute,
) -> Vec<u8> {
    let mut file = match esp
        .open(path, mode, attributes)
        .unwrap_or_else(|_| panic!("File {} not found", path))
        .into_type()
        .unwrap()
    {
        FileType::Regular(f) => f,
        _ => panic!("How do you expect me to load the {} folder?", path),
    };

    let mut buffer = vec![
        0;
        file.get_boxed_info::<FileInfo>()
            .unwrap_or_else(|_| panic!("Failed to get {} file info", path))
            .file_size()
            .try_into()
            .unwrap()
    ];

    file.read(&mut buffer)
        .unwrap_or_else(|_| panic!("Failed to read {}.", path));
    file.close();

    buffer
}
