// The app's real icon, as macOS shows it in Finder (NSWorkspace.icon(forFile:)), drawn at a small pixel size.
use objc2::rc::Retained;
use objc2::{AllocAnyThread, MainThreadMarker};
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSDeviceRGBColorSpace, NSGraphicsContext, NSWorkspace};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString};

/// PNG bytes, `px` pixels square. Must run on the main thread (sync Tauri commands do).
pub fn png(path: &str, px: isize) -> Option<Vec<u8>> {
    MainThreadMarker::new()?;
    unsafe {
        let img = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(path));
        let rep: Retained<NSBitmapImageRep> = NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            px,
            px,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            0,
            0,
        )?;
        let ctx = NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep)?;
        NSGraphicsContext::saveGraphicsState_class();
        NSGraphicsContext::setCurrentContext(Some(&ctx));
        let side = px as f64;
        img.drawInRect(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(side, side)));
        ctx.flushGraphics();
        NSGraphicsContext::restoreGraphicsState_class();
        let data = rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())?;
        Some(data.to_vec())
    }
}
