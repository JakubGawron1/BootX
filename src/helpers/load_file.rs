use ::alloc::*;
use uefi::prelude::*;
use uefi::proto::media::file::{Directory, File, FileAttribute, FileInfo, FileMode, FileType};

static mut ESP: Option<Directory> = None;

pub fn open_esp(image: Handle) {
    unsafe {
        let fs = uefi_services::system_table().as_mut().boot_services().get_image_file_system(image).unwrap().unwrap().get().as_mut().unwrap();
        ESP = Some(fs.open_volume().unwrap().unwrap());
    }
}

pub fn load_file(path: &str) -> vec::Vec<u8, alloc::Global> {
    let esp = unsafe { ESP.as_mut().unwrap() };
    let mut file = match esp
        .open(path, FileMode::Read, FileAttribute::empty())
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
    file.read(&mut buffer).expect_success(format!("Failed to read {}.", path).as_str());

    buffer
}
