use shadoword_core::{LocalService, ShadowwordConfig};

#[test]
fn auto_downloads_missing_model_and_loads() {
    let unique_suffix = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time before epoch")
            .as_nanos()
    );
    let models_dir = std::env::temp_dir().join(format!("shadoword-models-{unique_suffix}"));
    let default_file_name = "ggml-large-v3-turbo.bin";
    let model_path = models_dir.join(default_file_name);

    let _ = std::fs::remove_file(&model_path);
    let _ = std::fs::remove_dir_all(&models_dir);

    std::env::set_var("SHADOWORD_MODELS_DIR", &models_dir);

    let mut config = ShadowwordConfig::default();
    config.model_path = std::path::PathBuf::new();
    config.auto_download_model_if_missing = true;
    config.model_download_url = Some(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin"
            .to_string(),
    );

    let service = LocalService::new(config);
    service.preload().expect("preload should download and load model");
    assert!(service.is_loaded(), "service should be loaded after preload");
    assert!(model_path.exists(), "downloaded model file should exist");

    drop(service);
    std::env::remove_var("SHADOWORD_MODELS_DIR");
    let _ = std::fs::remove_file(&model_path);
    let _ = std::fs::remove_dir_all(&models_dir);
}
