//! Windows.Media.Ocr - the engine Windows has shipped since Windows 10.
//!
//! Needs no install and no key. See `mod.rs` for how it measured against
//! Tesseract on real screenshots.

use super::{finish, is_word, line_block, median_height};
use crate::local_ocr::OcrLine;
use windows::Graphics::Imaging::BitmapDecoder;
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

pub fn engine_available() -> bool {
    OcrEngine::TryCreateFromUserProfileLanguages()
        .map(|engine| engine.RecognizerLanguage().is_ok())
        .unwrap_or(false)
}

/// Puts the image bytes into a stream the imaging APIs accept, without
/// touching the filesystem - the Tesseract path has to write a temp file
/// because it shells out to a binary; this one does not.
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
        // Tesseract path uses, taken as a median over real words.
        let mut heights: Vec<f32> = Vec::new();
        let mut top = f32::MAX;
        for word in line.Words().map_err(|e| e.to_string())?.into_iter() {
            let rect = word.BoundingRect().map_err(|e| e.to_string())?;
            let word_text = word.Text().map_err(|e| e.to_string())?.to_string();
            if is_word(&word_text) {
                heights.push(rect.Height);
            }
            top = top.min(rect.Y);
        }

        blocks.push(line_block(
            text,
            if top == f32::MAX { 0.0 } else { top },
            median_height(heights),
        ));
    }

    finish(blocks)
}
