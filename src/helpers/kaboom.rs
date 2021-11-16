use alloc::boxed::Box;

pub fn fbinfo_from_gop(
    gop: &'static mut uefi::proto::console::gop::GraphicsOutput<'static>,
) -> Box<kaboom::tags::frame_buffer::FrameBufferInfo> {
    Box::new(kaboom::tags::frame_buffer::FrameBufferInfo {
        resolution: kaboom::tags::frame_buffer::ScreenRes::new(
            gop.current_mode_info().resolution(),
        ),
        pixel_format: match gop.current_mode_info().pixel_format() {
            uefi::proto::console::gop::PixelFormat::Rgb => {
                kaboom::tags::frame_buffer::PixelFormat::Rgb
            }
            uefi::proto::console::gop::PixelFormat::Bgr => {
                kaboom::tags::frame_buffer::PixelFormat::Bgr
            }
            uefi::proto::console::gop::PixelFormat::Bitmask => {
                kaboom::tags::frame_buffer::PixelFormat::Bitmask
            }
            _ => panic!("Blt-only mode not supported."),
        },
        pixel_bitmask: gop.current_mode_info().pixel_bitmask().map(|v| {
            kaboom::tags::frame_buffer::PixelBitmask {
                red: v.red,
                green: v.green,
                blue: v.blue,
            }
        }),
        pixels_per_scanline: gop.current_mode_info().stride(),
        base: gop.frame_buffer().as_mut_ptr() as *mut u32,
    })
}

pub fn mem_type_from_desc(
    desc: &uefi::table::boot::MemoryDescriptor,
) -> Option<core::cell::UnsafeCell<kaboom::tags::memory_map::MemoryEntry>> {
    let data = kaboom::tags::memory_map::MemoryData {
        base: desc.phys_start.try_into().unwrap(),
        length: (desc.page_count * 0x1000).try_into().unwrap(),
    };

    match desc.ty {
        uefi::table::boot::MemoryType::CONVENTIONAL => {
            Some(core::cell::UnsafeCell::new(
                kaboom::tags::memory_map::MemoryEntry::Usable(
                    kaboom::tags::memory_map::MemoryData {
                        base: desc.phys_start as usize,
                        length: desc.page_count as usize * 0x1000,
                    },
                ),
            ))
        }
        uefi::table::boot::MemoryType::LOADER_CODE | uefi::table::boot::MemoryType::LOADER_DATA => {
            Some(core::cell::UnsafeCell::new(
                kaboom::tags::memory_map::MemoryEntry::BootLoaderReclaimable(data),
            ))
        }
        uefi::table::boot::MemoryType::ACPI_RECLAIM => {
            Some(core::cell::UnsafeCell::new(
                kaboom::tags::memory_map::MemoryEntry::ACPIReclaimable(
                    kaboom::tags::memory_map::MemoryData {
                        base: desc.phys_start as usize,
                        length: desc.page_count as usize * 0x1000,
                    },
                ),
            ))
        }
        _ => None,
    }
}
