//! Configuration I/O. Missing files use defaults; other failures retain their context.
use anyhow::Context;
use serde::{de::DeserializeOwned, Serialize};
use std::{io::ErrorKind, path::Path};

pub(crate) fn load<T: DeserializeOwned>(path: &Path) -> anyhow::Result<Option<T>> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("Cannot read {}", path.display())),
    };
    serde_json::from_str(&contents)
        .map(Some)
        .with_context(|| format!("Invalid JSON in {}", path.display()))
}

pub(crate) fn save<T: Serialize>(path: &Path, settings: &T) -> anyhow::Result<()> {
    let contents = serde_json::to_string_pretty(settings)
        .with_context(|| format!("Cannot serialize settings for {}", path.display()))?;
    std::fs::write(path, contents).with_context(|| format!("Cannot write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn distinguishes_missing_invalid_and_io_errors_and_round_trips() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ethertia-settings-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let file = dir.join("settings.json");
        assert!(load::<serde_json::Value>(&file).unwrap().is_none());
        save(&file, &serde_json::json!({ "port": 4567 })).unwrap();
        assert_eq!(load::<serde_json::Value>(&file).unwrap().unwrap()["port"], 4567);
        std::fs::write(&file, "{").unwrap();
        let error = load::<serde_json::Value>(&file).unwrap_err().to_string();
        assert!(error.contains("Invalid JSON") && error.contains("settings.json"));
        assert!(load::<serde_json::Value>(&dir).is_err());
        assert!(save(&dir, &0).is_err());
        std::fs::remove_file(file).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
