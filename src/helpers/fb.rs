//! Copyright (c) VisualDevelopment 2021-2022.
//! This project is licensed by the Creative Commons Attribution-NoCommercial-NoDerivatives licence.

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
                    red: 0xFF000000,
                    green: 0x00FF0000,
                    blue: 0x0000FF00,
                    alpha: 0x000000FF,
                }
            }
            uefi::proto::console::gop::PixelFormat::Bgr => {
                PixelBitmask {
                    red: 0x0000FF00,
                    green: 0x00FF0000,
                    blue: 0xFF000000,
                    alpha: 0x000000FF,
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
