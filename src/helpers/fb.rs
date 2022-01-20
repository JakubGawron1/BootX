use alloc::boxed::Box;

use kaboom::tags::frame_buffer::{FrameBufferInfo, PixelBitmask, PixelFormat, ScreenRes};

pub fn fbinfo_from_gop(
    gop: &'static mut uefi::proto::console::gop::GraphicsOutput<'static>,
) -> Box<FrameBufferInfo> {
    Box::new(FrameBufferInfo {
        resolution: ScreenRes::new(gop.current_mode_info().resolution()),
        pixel_format: match gop.current_mode_info().pixel_format() {
            uefi::proto::console::gop::PixelFormat::Rgb => PixelFormat::Rgb,
            uefi::proto::console::gop::PixelFormat::Bgr => PixelFormat::Bgr,
            uefi::proto::console::gop::PixelFormat::Bitmask => PixelFormat::Bitmask,
            _ => panic!("Blt-only mode not supported."),
        },
        pixel_bitmask: match gop.current_mode_info().pixel_format() {
            uefi::proto::console::gop::PixelFormat::Rgb => {
                PixelBitmask {
                    red: 0x0000FF,
                    green: 0x00FF00,
                    blue: 0xFF0000,
                    alpha: 0xFF000000,
                }
            }
            uefi::proto::console::gop::PixelFormat::Bgr => {
                PixelBitmask {
                    red: 0xFF0000,
                    green: 0x00FF00,
                    blue: 0x0000FF,
                    alpha: 0xFF000000,
                }
            }
            uefi::proto::console::gop::PixelFormat::Bitmask => {
                gop.current_mode_info()
                    .pixel_bitmask()
                    .map(|v| {
                        PixelBitmask {
                            red: v.red,
                            green: v.green,
                            blue: v.blue,
                            alpha: v.reserved,
                        }
                    })
                    .unwrap()
            }
            _ => panic!("Blt-only mode not supported."),
        },
        pitch: gop.current_mode_info().stride(),
        base: gop.frame_buffer().as_mut_ptr() as *mut u32,
    })
}
