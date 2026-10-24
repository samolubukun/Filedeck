//! Pure-Rust office document parsing, conversion, and thumbnail rendering via libreoffice-pure.

use libreoffice_pure::{
    calc_convert_bytes, docx_to_pdf_bytes, docx_to_png_pages, impress_convert_bytes,
    pptx_to_pdf_bytes, pptx_to_png_pages, sniff_format_from_bytes, writer_convert_bytes,
};

/// Check if a filename corresponds to an office document format supported by pure Rust.
pub fn is_office_document(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.ends_with(".docx")
        || lower.ends_with(".doc")
        || lower.ends_with(".odt")
        || lower.ends_with(".xlsx")
        || lower.ends_with(".xls")
        || lower.ends_with(".ods")
        || lower.ends_with(".pptx")
        || lower.ends_with(".ppt")
        || lower.ends_with(".odp")
        || lower.ends_with(".rtf")
}

/// Convert an office document (DOCX, PPTX, XLSX, ODT) directly to PDF bytes in pure Rust.
pub fn convert_to_pdf(bytes: &[u8], filename: &str) -> Option<Vec<u8>> {
    let ext = filename.split('.').last()?.to_lowercase();

    match ext.as_str() {
        "docx" => docx_to_pdf_bytes(bytes).ok(),
        "doc" | "odt" | "rtf" => writer_convert_bytes(bytes, &ext, "pdf").ok(),
        "pptx" => pptx_to_pdf_bytes(bytes).ok(),
        "ppt" | "odp" => impress_convert_bytes(bytes, &ext, "pdf").ok(),
        "xlsx" | "xls" | "ods" => calc_convert_bytes(bytes, &ext, "pdf").ok(),
        _ => {
            if let Some(sniffed) = sniff_format_from_bytes(bytes) {
                match sniffed.as_str() {
                    "docx" => docx_to_pdf_bytes(bytes).ok(),
                    "pptx" => pptx_to_pdf_bytes(bytes).ok(),
                    "doc" | "odt" => writer_convert_bytes(bytes, &sniffed, "pdf").ok(),
                    "xlsx" | "ods" => calc_convert_bytes(bytes, &sniffed, "pdf").ok(),
                    _ => None,
                }
            } else {
                None
            }
        }
    }
}

/// Extract clean text or Markdown from an office document in pure Rust.
pub fn extract_text_content(bytes: &[u8], filename: &str) -> Option<String> {
    let ext = filename.split('.').last()?.to_lowercase();

    let text_bytes = match ext.as_str() {
        "docx" | "doc" | "odt" | "rtf" => {
            writer_convert_bytes(bytes, &ext, "txt")
                .or_else(|_| writer_convert_bytes(bytes, &ext, "md"))
                .ok()?
        }
        "xlsx" | "xls" | "ods" => calc_convert_bytes(bytes, &ext, "csv").ok()?,
        "pptx" | "ppt" | "odp" => impress_convert_bytes(bytes, &ext, "txt").ok()?,
        _ => return None,
    };

    String::from_utf8(text_bytes).ok()
}

/// Render the first page of an office document into a PNG image for instant thumbnails.
pub fn render_first_page_png(bytes: &[u8], filename: &str, dpi: u32) -> Option<Vec<u8>> {
    let ext = filename.split('.').last()?.to_lowercase();

    match ext.as_str() {
        "docx" => {
            let pages = docx_to_png_pages(bytes, dpi).ok()?;
            pages.into_iter().next()
        }
        "pptx" => {
            let pages = pptx_to_png_pages(bytes, dpi).ok()?;
            pages.into_iter().next()
        }
        _ => None,
    }
}
