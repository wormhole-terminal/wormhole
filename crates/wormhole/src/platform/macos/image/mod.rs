//! ImageIO's side of `platform::image`. The CoreFoundation, CoreGraphics and ImageIO calls are declared
//! here by hand (no crate wraps them); every object made is released on every path by a guard, and the
//! work is inside an autorelease pool, so a picture decoded a thousand times leaves nothing behind.

use std::ffi::c_void;

use objc2::rc::autoreleasepool;

use crate::platform::image::{Decoded, MAX_DECLARED_PIXELS, MAX_SIDE, Refused};

type CFRef = *const c_void;

#[repr(C)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

const CF_NUMBER_SINT32: isize = 3;
/// `kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little`: bytes in memory are B, G, R, A.
const BITMAP_INFO: u32 = 2 | (2 << 12);

#[link(name = "CoreFoundation", kind = "framework")]
#[allow(unsafe_code)]
unsafe extern "C" {
    static kCFTypeDictionaryKeyCallBacks: u8;
    static kCFTypeDictionaryValueCallBacks: u8;
    static kCFBooleanTrue: CFRef;
    static kCFBooleanFalse: CFRef;
    fn CFDataCreate(allocator: CFRef, bytes: *const u8, length: isize) -> CFRef;
    fn CFDictionaryCreate(
        allocator: CFRef,
        keys: *const CFRef,
        values: *const CFRef,
        count: isize,
        key_callbacks: *const u8,
        value_callbacks: *const u8,
    ) -> CFRef;
    fn CFDictionaryGetValue(dictionary: CFRef, key: CFRef) -> CFRef;
    fn CFNumberCreate(allocator: CFRef, kind: isize, value: *const c_void) -> CFRef;
    fn CFNumberGetValue(number: CFRef, kind: isize, value: *mut c_void) -> u8;
    fn CFRelease(object: CFRef);
}

#[link(name = "ImageIO", kind = "framework")]
#[allow(unsafe_code)]
unsafe extern "C" {
    static kCGImagePropertyPixelWidth: CFRef;
    static kCGImagePropertyPixelHeight: CFRef;
    static kCGImageSourceShouldCache: CFRef;
    static kCGImageSourceCreateThumbnailFromImageAlways: CFRef;
    static kCGImageSourceCreateThumbnailWithTransform: CFRef;
    static kCGImageSourceShouldCacheImmediately: CFRef;
    static kCGImageSourceThumbnailMaxPixelSize: CFRef;
    fn CGImageSourceCreateWithData(data: CFRef, options: CFRef) -> CFRef;
    fn CGImageSourceGetCount(source: CFRef) -> usize;
    fn CGImageSourceCopyPropertiesAtIndex(source: CFRef, index: usize, options: CFRef) -> CFRef;
    fn CGImageSourceCreateThumbnailAtIndex(source: CFRef, index: usize, options: CFRef) -> CFRef;
}

#[link(name = "CoreGraphics", kind = "framework")]
#[allow(unsafe_code)]
unsafe extern "C" {
    fn CGImageGetWidth(image: CFRef) -> usize;
    fn CGImageGetHeight(image: CFRef) -> usize;
    fn CGColorSpaceCreateDeviceRGB() -> CFRef;
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        space: CFRef,
        bitmap_info: u32,
    ) -> CFRef;
    fn CGContextDrawImage(context: CFRef, rect: CGRect, image: CFRef);
}

/// A CoreFoundation object (which CoreGraphics and ImageIO objects are too), released when it goes.
struct Owned(CFRef);

impl Owned {
    /// `None` for a null: the call that should have made it failed.
    fn new(object: CFRef) -> Option<Self> {
        (!object.is_null()).then_some(Self(object))
    }

    fn get(&self) -> CFRef {
        self.0
    }
}

impl Drop for Owned {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: the pointer is a non-null object this guard owns one reference to (a `Create` or
        // `Copy` result), released exactly once here.
        unsafe { CFRelease(self.0) }
    }
}

/// A `CFNumber` of `value`.
fn number(value: i32) -> Option<Owned> {
    // SAFETY: `value` outlives the call, which copies it; the kind says it is an `i32`.
    #[allow(unsafe_code)]
    let made =
        unsafe { CFNumberCreate(std::ptr::null(), CF_NUMBER_SINT32, std::ptr::from_ref(&value).cast()) };
    Owned::new(made)
}

/// A dictionary of CoreFoundation objects (keys are the framework's constant strings).
fn dictionary(entries: &[(CFRef, CFRef)]) -> Option<Owned> {
    let (keys, values): (Vec<CFRef>, Vec<CFRef>) = entries.iter().copied().unzip();
    // SAFETY: the arrays hold `entries.len()` valid objects that outlive the call, which retains them
    // (the standard callbacks); the callbacks are the framework's own statics.
    #[allow(unsafe_code)]
    let made = unsafe {
        CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            keys.len() as isize,
            &raw const kCFTypeDictionaryKeyCallBacks,
            &raw const kCFTypeDictionaryValueCallBacks,
        )
    };
    Owned::new(made)
}

/// An integer property of a properties dictionary (`PixelWidth`, `PixelHeight`).
fn property(properties: &Owned, key: CFRef) -> Option<u64> {
    // SAFETY: `properties` is a live dictionary and `key` a framework constant; the returned value is not
    // owned (the Get rule) and read before the dictionary goes.
    #[allow(unsafe_code)]
    let value = unsafe { CFDictionaryGetValue(properties.get(), key) };
    if value.is_null() {
        return None;
    }
    let mut out: i64 = 0;
    // SAFETY: `value` is the dictionary's number (these keys hold numbers) and `out` has room for an `i64`,
    // the kind asked for being a 64-bit one is not needed: a SInt32 read of a larger value fails, not overflows.
    #[allow(unsafe_code)]
    let ok = unsafe {
        CFNumberGetValue(value, 4 /* kCFNumberSInt64Type */, std::ptr::from_mut(&mut out).cast())
    };
    (ok != 0).then(|| u64::try_from(out).unwrap_or(0))
}

pub(crate) fn decode(bytes: &[u8]) -> Result<Decoded, Refused> {
    autoreleasepool(|_| decode_in_pool(bytes))
}

// SAFETY (the whole body): every pointer handed to a call is a live object this function holds a guard
// for, or a framework constant, and every size passed is the one of the buffer it describes.
#[allow(unsafe_code)]
fn decode_in_pool(bytes: &[u8]) -> Result<Decoded, Refused> {
    unsafe {
        let data = Owned::new(CFDataCreate(std::ptr::null(), bytes.as_ptr(), bytes.len() as isize))
            .ok_or(Refused::Unreadable)?;
        // Do not keep decoded pixels in the system's cache: this app holds the one copy it wants.
        let no_cache =
            dictionary(&[(kCGImageSourceShouldCache, kCFBooleanFalse)]).ok_or(Refused::Unreadable)?;
        let source =
            Owned::new(CGImageSourceCreateWithData(data.get(), no_cache.get())).ok_or(Refused::Unreadable)?;
        if CGImageSourceGetCount(source.get()) == 0 {
            return Err(Refused::Unreadable);
        }
        // The size the file declares, from its header, before anything is decoded.
        let properties = Owned::new(CGImageSourceCopyPropertiesAtIndex(source.get(), 0, std::ptr::null()))
            .ok_or(Refused::Unreadable)?;
        let width = property(&properties, kCGImagePropertyPixelWidth).ok_or(Refused::Unreadable)?;
        let height = property(&properties, kCGImagePropertyPixelHeight).ok_or(Refused::Unreadable)?;
        if width == 0 || height == 0 {
            return Err(Refused::Unreadable);
        }
        if width.saturating_mul(height) > MAX_DECLARED_PIXELS {
            return Err(Refused::TooLarge { width, height });
        }
        let max_side = number(MAX_SIDE as i32).ok_or(Refused::Unreadable)?;
        let options = dictionary(&[
            (kCGImageSourceCreateThumbnailFromImageAlways, kCFBooleanTrue),
            (kCGImageSourceCreateThumbnailWithTransform, kCFBooleanTrue),
            (kCGImageSourceShouldCacheImmediately, kCFBooleanTrue),
            (kCGImageSourceThumbnailMaxPixelSize, max_side.get()),
        ])
        .ok_or(Refused::Unreadable)?;
        let image = Owned::new(CGImageSourceCreateThumbnailAtIndex(source.get(), 0, options.get()))
            .ok_or(Refused::Unreadable)?;
        let (w, h) = (CGImageGetWidth(image.get()), CGImageGetHeight(image.get()));
        if w == 0 || h == 0 || w > MAX_SIDE as usize || h > MAX_SIDE as usize {
            return Err(Refused::Unreadable);
        }
        // Drawn into a bitmap we own, B, G, R, A with premultiplied alpha, which is what CoreGraphics offers.
        let mut pixels = vec![0u8; w * h * 4];
        let space = Owned::new(CGColorSpaceCreateDeviceRGB()).ok_or(Refused::Unreadable)?;
        let context = Owned::new(CGBitmapContextCreate(
            pixels.as_mut_ptr().cast(),
            w,
            h,
            8,
            w * 4,
            space.get(),
            BITMAP_INFO,
        ))
        .ok_or(Refused::Unreadable)?;
        let rect =
            CGRect { origin: CGPoint { x: 0.0, y: 0.0 }, size: CGSize { width: w as f64, height: h as f64 } };
        CGContextDrawImage(context.get(), rect, image.get());
        drop(context);
        Ok(Decoded { width: w as u32, height: h as u32, rgba: straight_rgba(pixels) })
    }
}

/// Premultiplied B, G, R, A in memory order, as straight R, G, B, A.
fn straight_rgba(mut pixels: Vec<u8>) -> Vec<u8> {
    for pixel in pixels.as_chunks_mut::<4>().0 {
        let alpha = u16::from(pixel[3]);
        let unpremultiply = |channel: u8| -> u8 {
            if alpha == 0 || alpha == 255 {
                channel
            } else {
                ((u16::from(channel) * 255 + alpha / 2) / alpha).min(255) as u8
            }
        };
        let (b, g, r) = (unpremultiply(pixel[0]), unpremultiply(pixel[1]), unpremultiply(pixel[2]));
        *pixel = [r, g, b, pixel[3]];
    }
    pixels
}

#[cfg(test)]
mod tests;
