//! Apple's Vision framework - the macOS equivalent of Windows.Media.Ocr.
//!
//! Present since macOS 10.15, needs no install and no key, which is the
//! whole point: a Mac user had to install Tesseract to use the app at all
//! while a Windows user did not.
//!
//! Two differences from the Windows engine, both handled here so nothing
//! downstream has to know which engine ran:
//!
//! - Vision reports normalised coordinates with the origin at the *bottom*
//!   left, so a line near the top of the image has a large y. Everything
//!   else in this codebase measures from the top in pixels, and the
//!   ordering of blocks is what the field heuristics are built on, so the
//!   box is flipped and scaled by the image's real height here.
//! - Vision groups by line and exposes one box per observation rather than
//!   one per word, so there are no word heights to take a median over. The
//!   line box is the line's height, which is what the median was
//!   approximating anyway; logo debris lands in its own observation rather
//!   than inflating a real line's height.

use super::{finish, line_block};
use crate::local_ocr::OcrLine;
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_foundation::{NSArray, NSData, NSDictionary};
use objc2_vision::{
    VNImageOption, VNImageRequestHandler, VNRecognizeTextRequest, VNRequest,
    VNRequestTextRecognitionLevel,
};

/// Vision ships with the OS, so the only real question is whether this is
/// a macOS build at all - which the module gate already answers.
pub fn engine_available() -> bool {
    true
}

/// The pixel height of the image, needed to turn Vision's normalised
/// boxes back into the units every other engine reports.
///
/// Reuses the `image` crate the Tesseract preprocessing already depends
/// on, and reads only the header rather than decoding the pixels.
fn image_height(bytes: &[u8]) -> Result<f32, String> {
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("Could not read that image: {e}"))?;
    let (_, height) = reader
        .into_dimensions()
        .map_err(|e| format!("Could not read that image: {e}"))?;
    Ok(height as f32)
}

pub fn recognise(image_bytes: &[u8]) -> Result<Vec<OcrLine>, String> {
    let height_px = image_height(image_bytes)?;

    let data = NSData::with_bytes(image_bytes);
    let options: Retained<NSDictionary<VNImageOption, objc2::runtime::AnyObject>> =
        NSDictionary::new();
    let handler = VNImageRequestHandler::initWithData_options(
        VNImageRequestHandler::alloc(),
        &data,
        &options,
    );

    let request = VNRecognizeTextRequest::new();
    // Accurate over Fast: this runs once, on a screenshot the user is
    // waiting on, and the whole point of the feature is not retyping.
    request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
    request.setUsesLanguageCorrection(true);

    // Two hops, not one: the chain is VNRecognizeTextRequest ->
    // VNImageBasedRequest -> VNRequest, and performRequests wants the base.
    let base: Retained<VNRequest> =
        Retained::into_super(Retained::into_super(request.clone()));
    let requests: Retained<NSArray<VNRequest>> = NSArray::from_retained_slice(&[base]);
    handler
        .performRequests_error(&requests)
        .map_err(|e| format!("macOS OCR failed: {e}"))?;

    let observations = request
        .results()
        .ok_or_else(|| "No text was found in that image.".to_string())?;

    let mut blocks = Vec::new();
    for observation in observations.iter() {
        let candidates = observation.topCandidates(1);
        let Some(best) = candidates.iter().next() else {
            continue;
        };
        let text = best.string().to_string();
        if text.trim().is_empty() {
            continue;
        }

        // The one call here that really is unsafe: it reads a CGRect out
        // of the observation with no checking on the Rust side.
        let box_ = unsafe { observation.boundingBox() };
        // Normalised, origin bottom-left. The distance from the top of the
        // image to the top of this line is what everything downstream sorts
        // and compares on.
        let top = (1.0 - (box_.origin.y + box_.size.height)) as f32 * height_px;
        let line_height = box_.size.height as f32 * height_px;

        blocks.push(line_block(text, top.max(0.0), line_height));
    }

    finish(blocks)
}
