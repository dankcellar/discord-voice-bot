use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use vosk::Model;

const MODEL_NAME: &str = "vosk-model-en-us-0.22";

pub fn load() -> Result<Arc<Model>, Box<dyn Error>> {
    let path = PathBuf::from(format!("models/{}", MODEL_NAME));

    if !path.is_dir() {
        return Err(format!("Model not found: {}", path.display()).into());
    }

    Model::new(path.to_string_lossy().as_ref())
        .map(Arc::new)
        .ok_or_else(|| format!("Failed to load model from {}", path.display()).into())
}
