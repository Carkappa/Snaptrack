//! The OCR engine the operating system already ships.
//!
//! Windows has had one since Windows 10 and it needs no install, which
//! removes the single biggest setup step this app had. Measured against
//! Tesseract on the same three screenshots it also reads them better for
//! this particular job: it produces no debris from a company logo - where
//! Tesseract emitted "| a Amazon" and "meS5 AtriCure, Inc." - and it read
//! the chips ("On-site", "Full-time") that Tesseract never managed. On one
//! card Tesseract returned nothing at all without preprocessing.
//!
//! It is not better at everything. Both misread small digits, and this one
//! drops the interpunct separators Tesseract keeps.
//!
//! Only Windows is implemented. macOS has an equivalent in the Vision
//! framework and Linux has none, so both fall back to Tesseract - the
//! provider only offers itself where it can actually run.

use crate::local_ocr::{OcrLine, SubLine};

/// Whether this build can use the system engine on this machine.
pub fn available() -> bool {
    #[cfg(windows)]
    {
        windows_impl::engine_available()
    }
    #[cfg(not(windows))]
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
        windows_impl::recognise(image_bytes).await
    }
    #[cfg(not(windows))]
    {
        let _ = image_bytes;
        Err("This build has no system OCR engine. Use Tesseract instead.".to_string())
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{OcrLine, SubLine};
    use windows::Graphics::Imaging::BitmapDecoder;
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

    pub fn engine_available() -> bool {
        OcrEngine::TryCreateFromUserProfileLanguages()
            .map(|engine| engine.RecognizerLanguage().is_ok())
            .unwrap_or(false)
    }

    /// Puts the image bytes into a stream the imaging APIs accept, without
    /// touching the filesystem - the Tesseract path has to write a temp
    /// file because it shells out to a binary; this one does not.
    async fn stream_from(bytes: &[u8]) -> windows::core::Result<InMemoryRandomAccessStream> {
        let stream = InMemoryRandomAccessStream::new()?;
        let writer = DataWriter::CreateDataWriter(&stream.GetOutputStreamAt(0)?)?;
        writer.WriteBytes(bytes)?;
        writer.StoreAsync()?.await?;
        writer.FlushAsync()?.await?;
        stream.Seek(0)?;
        Ok(stream)
    }

    pub async fn recognise(image_bytes: &[u8]) -> Result<Vec<OcrLine>, String> {
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()
            .map_err(|e| format!("Windows OCR is unavailable: {e}"))?;

        let stream = stream_from(image_bytes)
            .await
            .map_err(|e| format!("Could not read that image: {e}"))?;
        let decoder = BitmapDecoder::CreateAsync(&stream)
            .map_err(|e| format!("Could not decode that image: {e}"))?
            .await
            .map_err(|e| format!("Could not decode that image: {e}"))?;
        let bitmap = decoder
            .GetSoftwareBitmapAsync()
            .map_err(|e| format!("Could not decode that image: {e}"))?
            .await
            .map_err(|e| format!("Could not decode that image: {e}"))?;

        let result = engine
            .RecognizeAsync(&bitmap)
            .map_err(|e| format!("Windows OCR failed: {e}"))?
            .await
            .map_err(|e| format!("Windows OCR failed: {e}"))?;

        let mut blocks = Vec::new();
        for line in result.Lines().map_err(|e| e.to_string())?.into_iter() {
            let text = line.Text().map_err(|e| e.to_string())?.to_string();
            if text.trim().is_empty() {
                continue;
            }

            // Height comes from the word rectangles, the same measure the
            // Tesseract path uses. A median over real words rather than the
            // tallest, so one oversized glyph cannot claim to be a heading.
            let mut heights: Vec<f32> = Vec::new();
            let mut top = f32::MAX;
            for word in line.Words().map_err(|e| e.to_string())?.into_iter() {
                let rect = word.BoundingRect().map_err(|e| e.to_string())?;
                let word_text = word.Text().map_err(|e| e.to_string())?.to_string();
                if word_text.chars().filter(|c| c.is_alphanumeric()).count() >= 2 {
                    heights.push(rect.Height);
                }
                top = top.min(rect.Y);
            }
            heights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let height = heights.get(heights.len() / 2).copied().unwrap_or(0.0);

            blocks.push(OcrLine {
                top: if top == f32::MAX { 0.0 } else { top },
                height,
                sub_lines: vec![SubLine {
                    text: text.clone(),
                    height,
                }],
                text,
            });
        }

        if blocks.is_empty() {
            return Err("No text was found in that image.".to_string());
        }
        blocks.sort_by(|a, b| a.top.partial_cmp(&b.top).unwrap_or(std::cmp::Ordering::Equal));
        Ok(blocks)
    }
}
