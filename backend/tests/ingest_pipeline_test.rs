//! Integration tests for FileDeck's ingestion pipeline, date extraction, and office formats.

use filedeck_backend::ingest::dates::find_date;
use filedeck_backend::ingest::office::is_office_document;
use filedeck_backend::ingest::ConsumeRequest;
use sha2::{Digest, Sha256};

#[test]
fn test_find_date_iso_format() {
    assert_eq!(
        find_date("Invoice created on 2024-01-15 for subscription").as_deref(),
        Some("2024-01-15")
    );
    assert_eq!(
        find_date("Contract signed 2023-11-30.").as_deref(),
        Some("2023-11-30")
    );
    assert_eq!(
        find_date("Document dated 2019-08-04 in Berlin").as_deref(),
        Some("2019-08-04")
    );
}

#[test]
fn test_find_date_dot_numeric() {
    assert_eq!(
        find_date("Rechnungsdatum: 15.03.2024").as_deref(),
        Some("2024-03-15")
    );
    assert_eq!(
        find_date("Lieferung am 01.12.2022 erhalten").as_deref(),
        Some("2022-12-01")
    );
    assert_eq!(
        find_date("Datum 5.6.2021").as_deref(),
        Some("2021-06-05")
    );
}

#[test]
fn test_find_date_slash_numeric() {
    assert_eq!(
        find_date("Date: 25/12/2023").as_deref(),
        Some("2023-12-25")
    );
    assert_eq!(
        find_date("Issued on 5/3/2022 by provider").as_deref(),
        Some("2022-03-05")
    );
}

#[test]
fn test_find_date_textual_day_month_year() {
    assert_eq!(
        find_date("Payment received on 15 January 2024.").as_deref(),
        Some("2024-01-15")
    );
    assert_eq!(
        find_date("Scheduled for 3rd March 2023 at 10am").as_deref(),
        Some("2023-03-03")
    );
    assert_eq!(
        find_date("Effective 1st July 2021").as_deref(),
        Some("2021-07-01")
    );
    assert_eq!(
        find_date("Meeting held 22nd November 2020").as_deref(),
        Some("2020-11-22")
    );
    assert_eq!(
        find_date("Audit report 4th Sept. 2022").as_deref(),
        Some("2022-09-04")
    );
}

#[test]
fn test_find_date_textual_month_day_year() {
    assert_eq!(
        find_date("Invoice date: January 15, 2024").as_deref(),
        Some("2024-01-15")
    );
    assert_eq!(
        find_date("San Francisco, March 5th, 2023").as_deref(),
        Some("2023-03-05")
    );
    assert_eq!(
        find_date("Published October 31, 2022").as_deref(),
        Some("2022-10-31")
    );
    assert_eq!(
        find_date("Report from Feb 28, 2021").as_deref(),
        Some("2021-02-28")
    );
}

#[test]
fn test_find_date_precedence_earliest_position() {
    // Both 2022-01-01 and 2024-05-20 are in text; 2022-01-01 appears first
    let doc = "Referencing past agreement 2022-01-01, current invoice is 2024-05-20";
    assert_eq!(find_date(doc).as_deref(), Some("2022-01-01"));

    // Earliest text date wins over later ISO date
    let mixed = "Dated 15 March 2023, due on 2023-04-15";
    assert_eq!(find_date(mixed).as_deref(), Some("2023-03-15"));
}

#[test]
fn test_find_date_plausibility_and_bounds() {
    // Dates before 1970 should be ignored as implausible for modern scan archives
    assert_eq!(find_date("Founded in 1850-05-12 in London"), None);
    assert_eq!(find_date("Old record 12.04.1955"), None);

    // Completely unparseable text
    assert_eq!(find_date("No dates anywhere in this document text."), None);
    assert_eq!(find_date("Random numbers 123456789 and 99.99.9999"), None);
}

#[test]
fn test_is_office_document_detection() {
    // Modern Microsoft Office formats
    assert!(is_office_document("Financial_Model_2024.xlsx"));
    assert!(is_office_document("Quarterly_Report.docx"));
    assert!(is_office_document("Pitch_Deck.pptx"));

    // Legacy Microsoft Office formats
    assert!(is_office_document("Contract_V1.doc"));
    assert!(is_office_document("Budget.xls"));
    assert!(is_office_document("Slides.ppt"));

    // OpenDocument and Rich Text formats
    assert!(is_office_document("Statement.odt"));
    assert!(is_office_document("Spreadsheet.ods"));
    assert!(is_office_document("Presentation.odp"));
    assert!(is_office_document("Readme.rtf"));

    // Case insensitivity checks
    assert!(is_office_document("DOCUMENT.DOCX"));
    assert!(is_office_document("Spreadsheet.Xlsx"));
    assert!(is_office_document("SLIDES.PPTX"));

    // Non-office formats
    assert!(!is_office_document("document.pdf"));
    assert!(!is_office_document("scan.png"));
    assert!(!is_office_document("photo.jpg"));
    assert!(!is_office_document("archive.tar.gz"));
    assert!(!is_office_document("source.rs"));
    assert!(!is_office_document("script.py"));
    assert!(!is_office_document("schema.json"));
    assert!(!is_office_document("data.csv"));
}

#[test]
fn test_consume_request_initialization() {
    let req = ConsumeRequest {
        bytes: b"%PDF-1.4 test document content".to_vec(),
        filename: "test_invoice.pdf".to_string(),
        title: Some("Invoice 1001".to_string()),
        correspondent: Some(5),
        document_type: Some(2),
        storage_path: Some(1),
        tags: vec![10, 11, 12],
        created: Some("2024-02-14".to_string()),
        asn: Some(42001),
        custom_fields: vec![],
        source: "api_upload".to_string(),
        root_document_id: None,
        version_index: None,
        version_label: Some("Initial Draft".to_string()),
    };

    assert_eq!(req.filename, "test_invoice.pdf");
    assert_eq!(req.source, "api_upload");
    assert_eq!(req.tags.len(), 3);
    assert_eq!(req.asn, Some(42001));
    assert_eq!(req.bytes.len(), 30);
}

#[test]
fn test_sha256_document_checksumming() {
    let doc_content = b"FileDeck Document Integrity Check Payload";
    let mut hasher = Sha256::new();
    hasher.update(doc_content);
    let checksum = hex::encode(hasher.finalize());

    assert_eq!(checksum.len(), 64);
    
    // Identical content produces identical checksum
    let mut hasher2 = Sha256::new();
    hasher2.update(doc_content);
    assert_eq!(hex::encode(hasher2.finalize()), checksum);

    // Modified content produces different checksum
    let mut hasher3 = Sha256::new();
    hasher3.update(b"FileDeck Document Integrity Check Payload Modified");
    assert_ne!(hex::encode(hasher3.finalize()), checksum);
}
