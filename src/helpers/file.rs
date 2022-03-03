//! Copyright (c) VisualDevelopment 2021-2022.
//! This project is licensed by the Creative Commons Attribution-NoCommercial-NoDerivatives licence.

use alloc::{format, vec, vec::Vec};

use uefi::{
    prelude::*,
    proto::media::file::{Directory, File, FileAttribute, FileInfo, FileMode, FileType},
};

pub fn open_esp(image: Handle) -> Directory {
    unsafe {
        let fs = uefi_services::system_table()
            .as_mut()
            .boot_services()
            .get_image_file_system(image)
            .expect_success("Failed to get ESP")
            .interface
            .get()
            .as_mut()
            .unwrap();

        fs.open_volume().expect("Failed to open volume.").unwrap()
    }
}

pub fn load(esp: &mut Directory, path: &str, mode: FileMode, attributes: FileAttribute) -> Vec<u8> {
    let mut file = match esp
        .open(path, mode, attributes)
        .expect_success(format!("File {} not found", path).as_str())
        .into_type()
        .unwrap()
        .unwrap()
    {
        FileType::Regular(f) => f,
        _ => panic!("How do you expect me to load the {} folder?", path),
    };

    let mut buffer = vec![
        0;
        file.get_boxed_info::<FileInfo>()
            .expect_success(format!("Failed to get {} file info", path).as_str())
            .file_size()
            .try_into()
            .unwrap()
    ];

    file.read(&mut buffer)
        .expect_success(format!("Failed to read {}.", path).as_str());
    file.close();

    buffer
}
