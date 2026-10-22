//! CoreText's side of `platform::fonts`: a family's traits, and font files registered for this
//! process.

use std::path::Path;

use crate::platform::fonts::Traits;

/// Classify a family by its default face. GPUI doesn't expose font traits,
/// so this asks CoreText (thread-safe, and doesn't load the font into GPUI's
/// caches). The fixed-pitch flag alone misses some coding fonts, so equal
/// advances for `iMW0` count as monospace too.
#[allow(unsafe_code)]
pub fn classify(family: &str) -> Option<Traits> {
    use std::ptr::NonNull;

    use objc2_core_foundation::{CFDictionary, CFString, CFType, CGSize};
    use objc2_core_text::{
        CTFont, CTFontDescriptor, CTFontOrientation, CTFontStylisticClass, CTFontSymbolicTraits,
        kCTFontFamilyNameAttribute,
    };

    let name = CFString::from_str(family);
    // SAFETY: `kCTFontFamilyNameAttribute` is an immutable CoreText constant.
    // The dictionary holds CF types, as `CTFontDescriptorCreateWithAttributes`
    // expects; the font keeps its own references. The glyph and advance
    // buffers have exactly `count` elements and outlive the calls.
    unsafe {
        let key: &CFString = kCTFontFamilyNameAttribute;
        let value: &CFType = name.as_ref();
        let attributes = CFDictionary::<CFString, CFType>::from_slices(&[key], &[value]);
        let descriptor = CTFontDescriptor::with_attributes(attributes.as_opaque());
        let font = CTFont::with_font_descriptor(&descriptor, 12.0, std::ptr::null());
        // CoreText substitutes a default font for a family it can't find.
        if font.family_name().to_string() != family {
            return None;
        }
        let traits = font.symbolic_traits();
        let class = traits.0 & CTFontSymbolicTraits::ClassMaskTrait.0;
        let symbolic = class == CTFontStylisticClass::SymbolicClass.0
            || class == CTFontStylisticClass::OrnamentalsClass.0;

        let mut chars: [u16; 4] = b"iMW0".map(u16::from);
        let mut glyphs = [0u16; 4];
        let mapped = font.glyphs_for_characters(
            NonNull::from(&mut chars).cast(),
            NonNull::from(&mut glyphs).cast(),
            4,
        );
        let mut advances = [CGSize { width: 0.0, height: 0.0 }; 4];
        let equal_advances = mapped && {
            font.advances_for_glyphs(
                CTFontOrientation::Horizontal,
                NonNull::from(&mut glyphs).cast(),
                advances.as_mut_ptr(),
                4,
            );
            advances[0].width > 0.0 && advances.iter().all(|a| (a.width - advances[0].width).abs() < 0.01)
        };
        Some(Traits {
            monospace: traits.contains(CTFontSymbolicTraits::TraitMonoSpace) || equal_advances,
            symbolic: symbolic || !mapped,
        })
    }
}

/// Make a font file's faces available to this process (or stop that), like
/// an installed font but without touching the system. User fonts go through
/// CoreText rather than GPUI's `add_fonts`, which reads only a collection's
/// first face and can't let go of a font whose file was removed or replaced.
#[allow(unsafe_code)]
pub fn register(path: &Path, register: bool) -> anyhow::Result<()> {
    use std::ptr::NonNull;

    use objc2_core_foundation::{CFError, CFRetained, CFURL};
    use objc2_core_text::{
        CTFontManagerError, CTFontManagerRegisterFontsForURL, CTFontManagerScope,
        CTFontManagerUnregisterFontsForURL,
    };

    let url = CFURL::from_file_path(path).ok_or_else(|| anyhow::anyhow!("not a file path"))?;
    let mut error: *mut CFError = std::ptr::null_mut();
    // SAFETY: `url` is a valid CFURL for the duration of the call, and `error`
    // points to a local null pointer that CoreText only sets (to an owned,
    // +1 CFError) when the call fails; it's adopted exactly once below.
    let (ok, error) = unsafe {
        let ok = if register {
            CTFontManagerRegisterFontsForURL(&url, CTFontManagerScope::Process, &mut error)
        } else {
            CTFontManagerUnregisterFontsForURL(&url, CTFontManagerScope::Process, &mut error)
        };
        (ok, NonNull::new(error).map(|error| CFRetained::from_raw(error)))
    };
    if ok {
        return Ok(());
    }
    let Some(error) = error else { anyhow::bail!("CoreText refused the file") };
    let code = error.code();
    // Loading a file twice, or dropping one that wasn't loaded, changes nothing.
    if code == CTFontManagerError::AlreadyRegistered.0 || code == CTFontManagerError::NotRegistered.0 {
        return Ok(());
    }
    let description = error.description().map(|d| d.to_string()).unwrap_or_default();
    anyhow::bail!("CoreText error {code}: {description}")
}
