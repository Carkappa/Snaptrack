//! The OCR engine the operating system already ships.
//!
//! This removes the single biggest setup step the app had. Measured
//! against Tesseract on the same three screenshots, the Windows engine
//! also reads them better for this particular job: it produces no debris
//! from a company logo - where Tesseract emitted "| a Amazon" and "meS5
//! AtriCure, Inc." - and it read the chips ("On-site", "Full-time") that
//! Tesseract never managed. On one card Tesseract returned nothing at all
//! without preprocessing.
//!
//! It is not better at everything. Both misread small digits, and the
//! system engines drop the interpunct separators Tesseract keeps.
//!
//! One engine per platform, each in its own file, both returning the same
//! `Vec<OcrLine>` the Tesseract path produces - so the field heuristics,
//! click-to-fill and correction-learning are unchanged behind any of them:
//!
//! - `windows_engine` - Windows.Media.Ocr, present since Windows 10.
//! - `macos_engine` - the Vision framework, present since macOS 10.15.
//!
//! Linux has no equivalent and falls back to Tesseract. `available()`
//! gates whether the method is offered at all, so nobody is shown a
//! choice their machine cannot honour.

use crate::local_ocr::{OcrLine, SubLine};

#[cfg(target_os = "macos")]
mod macos_engine;
#[cfg(windows)]
mod windows_engine;

/// Whether this build can use the system engine on this machine.
pub fn available() -> bool {
    #[cfg(windows)]
    {
        windows_engine::engine_available()
    }
    #[cfg(target_os = "macos")]
    {
        macos_engine::engine_available()
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        false
    }
}

/// Reads an image with the system engine, returning the same blocks the
/// Tesseract path produces so everything downstream - the field
/// heuristics, click-to-fill, learning from corrections - is unchanged.
pub async fn run(image_bytes: &[u8]) -> Result<Vec<OcrLine>, String> {
    #[cfg(windows)]
    {
        windows_engine::recognise(image_bytes).await
    }
    #[cfg(target_os = "macos")]
    {
        macos_engine::recognise(image_bytes)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = image_bytes;
        Err("This build has no system OCR engine. Use Tesseract instead.".to_string())
    }
}

/// Builds the block an engine reports for one line of text.
///
/// Shared so the two engines cannot drift on the shape they hand back.
/// `height` is the line's text height in pixels and `top` its distance
/// from the top of the image, which is the order and the scale every
/// heuristic downstream is written against.
pub(crate) fn line_block(text: String, top: f32, height: f32) -> OcrLine {
    OcrLine {
        top,
        height,
        sub_lines: vec![SubLine {
            text: text.clone(),
            height,
        }],
        text,
    }
}

/// Puts the blocks in reading order and refuses an image with no text,
/// rather than letting an empty result look like a successful read of an
/// empty posting.
pub(crate) fn finish(mut blocks: Vec<OcrLine>) -> Result<Vec<OcrLine>, String> {
    if blocks.is_empty() {
        return Err("No text was found in that image.".to_string());
    }
    blocks.sort_by(|a, b| a.top.partial_cmp(&b.top).unwrap_or(std::cmp::Ordering::Equal));
    Ok(blocks)
}

/// The median of some word heights, or 0.0 if there are none.
///
/// A median rather than the tallest, because one oversized glyph - the
/// single letters a logo decomposes into, most often - would otherwise
/// let a decorative row claim to be a heading. That was a real bug: the
/// job title lost to the company logo above it on every Amazon card.
pub(crate) fn median_height(mut heights: Vec<f32>) -> f32 {
    if heights.is_empty() {
        return 0.0;
    }
    heights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    heights[heights.len() / 2]
}

/// Whether a token is a real word rather than logo debris or a bullet.
pub(crate) fn is_word(text: &str) -> bool {
    text.chars().filter(|c| c.is_alphanumeric()).count() >= 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_median_ignores_one_oversized_glyph() {
        // The logo case: a row of normal words with one huge letter in it
        // must still report the height of the words.
        assert_eq!(median_height(vec![12.0, 11.0, 90.0]), 12.0);
    }

    #[test]
    fn a_median_of_nothing_is_zero_rather_than_a_panic() {
        assert_eq!(median_height(vec![]), 0.0);
    }

    #[test]
    fn single_characters_and_bullets_are_not_words() {
        assert!(!is_word("a"));
        assert!(!is_word("-"));
        assert!(!is_word(""));
        assert!(is_word("at"));
        assert!(is_word("2026"));
    }

    #[test]
    fn blocks_come_back_in_reading_order() {
        let blocks = vec![
            line_block("second".into(), 40.0, 10.0),
            line_block("first".into(), 10.0, 20.0),
        ];
        let sorted = finish(blocks).unwrap();
        assert_eq!(sorted[0].text, "first");
        assert_eq!(sorted[1].text, "second");
    }

    #[test]
    fn an_image_with_no_text_is_an_error_not_an_empty_read() {
        assert!(finish(Vec::new()).is_err());
    }

    #[test]
    fn a_block_carries_its_text_into_its_only_sub_line() {
        let block = line_block("Robotics Engineer".into(), 5.0, 21.0);
        assert_eq!(block.sub_lines.len(), 1);
        assert_eq!(block.sub_lines[0].text, "Robotics Engineer");
        assert_eq!(block.sub_lines[0].height, 21.0);
    }
}
