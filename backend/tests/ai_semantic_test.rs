//! Integration tests for FileDeck's local-first AI chunking, embedding serialization, and cosine similarity.

use filedeck_backend::ai::{chunk_text, cosine, CHUNK_CHARS, MAX_CHUNKS};

#[test]
fn test_chunk_text_empty_and_short() {
    assert!(chunk_text("", "").is_empty());
    assert!(chunk_text("   ", "   ").is_empty());

    let single = chunk_text("Doc Title", "Short body content here");
    assert_eq!(single.len(), 1);
    assert!(single[0].contains("Doc Title"));
    assert!(single[0].contains("Short body content here"));
}

#[test]
fn test_chunk_text_boundary_splitting() {
    // Generate text that exceeds CHUNK_CHARS (1000 chars)
    let words = vec!["FileDeck"; 200];
    let body = words.join(" ");
    assert!(body.len() > CHUNK_CHARS);

    let chunks = chunk_text("Large Document", &body);
    assert!(chunks.len() > 1);

    for (idx, c) in chunks.iter().enumerate() {
        assert!(c.len() <= CHUNK_CHARS + 50, "Chunk {} exceeds reasonable bounds: {}", idx, c.len());
        // Verify no word got sliced in half
        assert!(!c.ends_with("FileD"));
    }
}

#[test]
fn test_chunk_text_max_chunks_limit() {
    // Generate huge text that would produce > 50 chunks
    let words = vec!["document_term"; 4000];
    let body = words.join(" ");

    let chunks = chunk_text("Massive Document", &body);
    assert!(chunks.len() <= MAX_CHUNKS);
    assert_eq!(chunks.len(), MAX_CHUNKS);
}

#[test]
fn test_cosine_similarity_identical_vectors() {
    let a = vec![1.0, 2.0, 3.0, 4.0];
    let b = vec![1.0, 2.0, 3.0, 4.0];
    let sim = cosine(&a, &b);
    assert!((sim - 1.0).abs() < 1e-5);
}

#[test]
fn test_cosine_similarity_orthogonal_vectors() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    let sim = cosine(&a, &b);
    assert!((sim - 0.0).abs() < 1e-5);
}

#[test]
fn test_cosine_similarity_opposite_vectors() {
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![-1.0, -2.0, -3.0];
    let sim = cosine(&a, &b);
    assert!((sim - (-1.0)).abs() < 1e-5);
}

#[test]
fn test_cosine_similarity_zero_vectors_no_nan() {
    let zero = vec![0.0, 0.0, 0.0];
    let norm = vec![1.0, 2.0, 3.0];

    // Zero vector must return 0.0 and never NaN
    let sim1 = cosine(&zero, &norm);
    assert!(!sim1.is_nan());
    assert_eq!(sim1, 0.0);

    let sim2 = cosine(&zero, &zero);
    assert!(!sim2.is_nan());
    assert_eq!(sim2, 0.0);
}

#[test]
fn test_cosine_similarity_mismatched_lengths() {
    let a = vec![1.0, 2.0];
    let b = vec![1.0, 2.0, 3.0];
    assert_eq!(cosine(&a, &b), 0.0);

    let empty: Vec<f32> = vec![];
    assert_eq!(cosine(&empty, &empty), 0.0);
}

#[test]
fn test_cosine_similarity_high_dimensional_embeddings() {
    // 384-dimensional vector standard for MiniLM-L6-v2 embeddings
    let vec_a: Vec<f32> = (0..384).map(|i| (i as f32).sin()).collect();
    let vec_b: Vec<f32> = (0..384).map(|i| (i as f32).sin()).collect();

    let sim = cosine(&vec_a, &vec_b);
    assert!((sim - 1.0).abs() < 1e-4);

    let vec_c: Vec<f32> = (0..384).map(|i| (i as f32).cos()).collect();
    let sim_ac = cosine(&vec_a, &vec_c);
    assert!(sim_ac >= -1.0 && sim_ac <= 1.0);
}

#[test]
fn test_float_byte_serialization_roundtrip() {
    let floats: Vec<f32> = vec![0.12345, -99.88, 3.14159, 1000.0, 0.0];
    
    // Convert to little-endian bytes
    let blob: Vec<u8> = floats.iter().flat_map(|f| f.to_le_bytes()).collect();
    assert_eq!(blob.len(), floats.len() * 4);

    // Reconstruct from little-endian bytes
    let reconstructed: Vec<f32> = blob
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    assert_eq!(floats.len(), reconstructed.len());
    for (orig, recon) in floats.iter().zip(reconstructed.iter()) {
        assert!((orig - recon).abs() < 1e-6);
    }
}
