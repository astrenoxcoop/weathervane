#[cfg(feature = "reload")]
pub mod reload_env {
    use std::path::PathBuf;

    use minijinja::{path_loader, Environment};
    use minijinja_autoreload::AutoReloader;

    pub fn build_env(http_external: &str, version: &str) -> AutoReloader {
        let http_external = http_external.to_string();
        let version = version.to_string();
        AutoReloader::new(move |notifier| {
            let template_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates");
            let mut env = Environment::new();
            env.set_trim_blocks(true);
            env.set_lstrip_blocks(true);
            env.add_global("base", http_external.clone());
            env.add_global("version", version.clone());
            env.set_loader(path_loader(&template_path));
            notifier.set_fast_reload(true);
            notifier.watch_path(&template_path, true);
            Ok(env)
        })
    }
}

#[cfg(feature = "embed")]
pub mod embed_env {
    use minijinja::Environment;

    pub fn build_env(http_external: String, version: String) -> Environment<'static> {
        let mut env = Environment::new();
        env.set_trim_blocks(true);
        env.set_lstrip_blocks(true);
        env.add_global("base", http_external.clone());
        env.add_global("version", version.clone());
        minijinja_embed::load_templates!(&mut env);
        env
    }
}
