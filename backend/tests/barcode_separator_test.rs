//! Tests for barcode and separator sheet detection and document splitting.

use filedeck_backend::extraction::barcode::{
    calculate_code39_mod43, split_scanned_batch, verify_code128_subset_b, verify_code39_checksum,
    BarcodeType, DetectedBarcode, PatchCodeType,
};

#[test]
fn test_code39_mod43_checksums() {
    // Known standard Code 39 Mod-43 vectors:
    // "12345" -> Sum: 1+2+3+4+5 = 15 -> Character 15 = 'F'
    assert_eq!(calculate_code39_mod43("12345"), Some('F'));
    assert!(verify_code39_checksum("12345F"));
    assert!(!verify_code39_checksum("12345E"));

    // "TEST" -> T(29) + E(14) + S(28) + T(29) = 100 % 43 = 14 -> 'E'
    assert_eq!(calculate_code39_mod43("TEST"), Some('E'));
    assert!(verify_code39_checksum("*TESTE*"));

    // Invalid character
    assert_eq!(calculate_code39_mod43("test@invalid"), None);
}

#[test]
fn test_code128_subset_b_checksums() {
    // Checksum calculation verification:
    // Start B (104) + "A"(33 * 1 = 33) = 137 % 103 = 34
    assert!(verify_code128_subset_b("A", 34));
    assert!(!verify_code128_subset_b("A", 35));
}

#[test]
fn test_patch_code_classification() {
    assert_eq!(PatchCodeType::from_str_loose("PATCH-T"), Some(PatchCodeType::PatchT));
    assert_eq!(PatchCodeType::from_str_loose("patch t"), Some(PatchCodeType::PatchT));
    assert_eq!(PatchCodeType::from_str_loose("PATCH1"), Some(PatchCodeType::Patch1));
    assert_eq!(PatchCodeType::from_str_loose("PATCH-2"), Some(PatchCodeType::Patch2));
    assert_eq!(PatchCodeType::from_str_loose("PATCH-3"), Some(PatchCodeType::Patch3));
    assert_eq!(PatchCodeType::from_str_loose("PATCH-4"), Some(PatchCodeType::Patch4));
    assert_eq!(PatchCodeType::from_str_loose("PATCH-6"), Some(PatchCodeType::Patch6));
    assert_eq!(PatchCodeType::from_str_loose("OTHER"), None);

    assert_eq!(PatchCodeType::PatchT.as_str(), "PATCH-T");
    assert_eq!(PatchCodeType::Patch1.as_str(), "PATCH-1");
}

#[test]
fn test_scanned_batch_splitting_scenarios() {
    // Scenario 1: Clean multi-doc batch with Patch T sheets
    // 15 pages total. Pages 5 and 11 are Patch T separator sheets.
    let barcodes = vec![
        DetectedBarcode {
            barcode_type: BarcodeType::PatchCode,
            raw_data: "PATCH-T".to_string(),
            page_number: 5,
            confidence: 0.99,
            bbox: Some([0.1, 0.4, 0.2, 0.6]),
            is_separator: true,
        },
        DetectedBarcode {
            barcode_type: BarcodeType::PatchCode,
            raw_data: "PATCH-T".to_string(),
            page_number: 11,
            confidence: 0.99,
            bbox: Some([0.1, 0.4, 0.2, 0.6]),
            is_separator: true,
        },
    ];

    let segments = split_scanned_batch(15, &barcodes, true);
    assert_eq!(segments.len(), 3);

    // Segment 1: Pages 1-4
    assert_eq!(segments[0].document_index, 1);
    assert_eq!(segments[0].start_page, 1);
    assert_eq!(segments[0].end_page, 4);
    assert_eq!(segments[0].page_count, 4);

    // Segment 2: Pages 6-10 (Page 5 discarded)
    assert_eq!(segments[1].document_index, 2);
    assert_eq!(segments[1].start_page, 6);
    assert_eq!(segments[1].end_page, 10);
    assert_eq!(segments[1].page_count, 5);

    // Segment 3: Pages 12-15 (Page 11 discarded)
    assert_eq!(segments[2].document_index, 3);
    assert_eq!(segments[2].start_page, 12);
    assert_eq!(segments[2].end_page, 15);
    assert_eq!(segments[2].page_count, 4);

    // Scenario 2: Batch without any separator sheets (should return single segment)
    let single_segments = split_scanned_batch(8, &[], true);
    assert_eq!(single_segments.len(), 1);
    assert_eq!(single_segments[0].start_page, 1);
    assert_eq!(single_segments[0].end_page, 8);
    assert_eq!(single_segments[0].page_count, 8);

    // Scenario 3: Empty batch (0 pages)
    assert!(split_scanned_batch(0, &[], true).is_empty());
}
