use anyhow::Result;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn get_type(path: &Path) -> Result<String> {
    // Open the file and read the first 512 bytes for type detection
    let mut file = File::open(path)?;
    let mut buffer = vec![0u8; 512];
    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);
    
    // Use infer crate to detect MIME type
    if let Some(kind) = infer::get(&buffer) {
        Ok(kind.mime_type().to_string())
    } else {
        // Fall back to extension-based detection
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            Ok(match ext_str.as_str() {
                "txt" => "text/plain",
                "html" | "htm" => "text/html",
                "css" => "text/css",
                "js" => "application/javascript",
                "json" => "application/json",
                "xml" => "application/xml",
                "pdf" => "application/pdf",
                "zip" => "application/zip",
                "tar" => "application/x-tar",
                "gz" => "application/gzip",
                _ => "application/octet-stream",
            }.to_string())
        } else {
            Ok("application/octet-stream".to_string())
        }
    }
}

#[allow(dead_code)]
pub fn is_image(mime_type: &str) -> bool {
    mime_type.starts_with("image/")
}

#[allow(dead_code)]
pub fn is_video(mime_type: &str) -> bool {
    mime_type.starts_with("video/")
}

#[allow(dead_code)]
pub fn is_audio(mime_type: &str) -> bool {
    mime_type.starts_with("audio/")
}

#[allow(dead_code)]
pub fn is_text(mime_type: &str) -> bool {
    mime_type.starts_with("text/")
}

#[allow(dead_code)]
pub fn is_document(mime_type: &str) -> bool {
    matches!(mime_type,
        "application/pdf" |
        "application/msword" |
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" |
        "application/vnd.ms-excel" |
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" |
        "application/vnd.ms-powerpoint" |
        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_type_with_known_files() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test text file
        let txt_path = temp_dir.path().join("test.txt");
        fs::write(&txt_path, "Hello, world!").unwrap();
        assert_eq!(get_type(&txt_path).unwrap(), "text/plain");
        
        // Test HTML file
        let html_path = temp_dir.path().join("test.html");
        fs::write(&html_path, "<html><body>Test</body></html>").unwrap();
        assert_eq!(get_type(&html_path).unwrap(), "text/html");
        
        // Test JSON file
        let json_path = temp_dir.path().join("test.json");
        fs::write(&json_path, r#"{"key": "value"}"#).unwrap();
        assert_eq!(get_type(&json_path).unwrap(), "application/json");
        
        // Test file with no extension
        let no_ext_path = temp_dir.path().join("noext");
        fs::write(&no_ext_path, "some content").unwrap();
        assert_eq!(get_type(&no_ext_path).unwrap(), "application/octet-stream");
    }
    
    #[test]
    fn test_get_type_with_binary_content() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create a PNG file (minimal valid PNG header)
        let png_path = temp_dir.path().join("test.png");
        let png_header = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        fs::write(&png_path, png_header).unwrap();
        assert_eq!(get_type(&png_path).unwrap(), "image/png");
        
        // Create a JPEG file (minimal valid JPEG header)
        let jpg_path = temp_dir.path().join("test.jpg");
        let jpg_header = vec![0xFF, 0xD8, 0xFF, 0xE0];
        fs::write(&jpg_path, jpg_header).unwrap();
        assert_eq!(get_type(&jpg_path).unwrap(), "image/jpeg");
    }

    #[test]
    fn test_is_image() {
        assert!(is_image("image/png"));
        assert!(is_image("image/jpeg"));
        assert!(is_image("image/gif"));
        assert!(!is_image("text/plain"));
        assert!(!is_image("application/pdf"));
    }

    #[test]
    fn test_is_video() {
        assert!(is_video("video/mp4"));
        assert!(is_video("video/mpeg"));
        assert!(is_video("video/webm"));
        assert!(!is_video("image/png"));
        assert!(!is_video("audio/mp3"));
    }

    #[test]
    fn test_is_audio() {
        assert!(is_audio("audio/mp3"));
        assert!(is_audio("audio/wav"));
        assert!(is_audio("audio/ogg"));
        assert!(!is_audio("video/mp4"));
        assert!(!is_audio("text/plain"));
    }

    #[test]
    fn test_is_text() {
        assert!(is_text("text/plain"));
        assert!(is_text("text/html"));
        assert!(is_text("text/css"));
        assert!(!is_text("image/png"));
        assert!(!is_text("application/pdf"));
    }

    #[test]
    fn test_is_document() {
        assert!(is_document("application/pdf"));
        assert!(is_document("application/msword"));
        assert!(is_document("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
        assert!(!is_document("text/plain"));
        assert!(!is_document("image/png"));
    }
}