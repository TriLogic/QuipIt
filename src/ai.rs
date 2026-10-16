mod chat;
pub(crate) use chat::Chat;

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
enum Provider {
    #[default]
    OpenAi,
    Gemini,
    Grok,
}

impl Provider {
    const ALL: [Self; 3] = [Self::OpenAi, Self::Gemini, Self::Grok];

    fn name(self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI",
            Self::Gemini => "Gemini",
            Self::Grok => "Grok",
        }
    }

    fn index(self) -> usize {
        self as usize
    }

    fn entry(self) -> Result<keyring::Entry, String> {
        keyring::Entry::new("QuipIt.AI", self.name())
            .map_err(|_| "Could not open the OS credential store.".into())
    }
}

// Only these preferences are serialized. API keys never enter the settings file.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    selected: Option<Provider>,
    providers: [ProviderPreferences; 3],
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct ProviderPreferences {
    model: String,
    remember: bool,
}

enum ProviderResult {
    Connection,
    Models(Vec<String>),
}

type TestResult = Arc<Mutex<Option<Result<ProviderResult, String>>>>;

#[derive(Clone)]
struct Configuration {
    provider: Provider,
    key: String,
    model: String,
    remember: bool,
    message: String,
    pending: Option<TestResult>,
    models: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct Settings {
    preferences: Preferences,
    keys: [String; 3],
    draft: Provider,
    open: bool,
    configuration: Option<Configuration>,
    message: String,
}

impl Default for Settings {
    fn default() -> Self {
        let loaded = preferences_path().and_then(|path| match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| {
                "AI settings could not be read. Configure your providers again.".into()
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Preferences::default()),
            Err(_) => Err("Could not read AI settings.".into()),
        });
        let (preferences, message) = match loaded {
            Ok(preferences) => (preferences, String::new()),
            Err(message) => (Preferences::default(), message),
        };
        Self {
            draft: preferences.selected.unwrap_or_default(),
            preferences,
            keys: Default::default(),
            open: false,
            configuration: None,
            message,
        }
    }
}

fn preferences_path() -> Result<PathBuf, String> {
    directories::ProjectDirs::from("", "", "QuipIt")
        .map(|dirs| dirs.config_dir().join("ai.json"))
        .ok_or_else(|| "Could not find the application settings folder.".into())
}

fn write_preferences(preferences: &Preferences, path: &std::path::Path) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid settings path.")?;
    std::fs::create_dir_all(parent).map_err(|_| "Could not create the settings folder.")?;
    let mut file =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| "Could not write AI settings.")?;
    serde_json::to_writer_pretty(file.as_file_mut(), preferences)
        .map_err(|_| "Could not write AI settings.")?;
    file.persist(path)
        .map_err(|_| "Could not save AI settings.")?;
    Ok(())
}

fn store_key(entry: &keyring::Entry, key: &str, remember: bool) -> Result<(), String> {
    if remember {
        entry.set_password(key).map_err(|_| {
            "Could not save the key. Unlock the OS credential store or turn off Remember key."
                .into()
        })
    } else {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(
                "Could not remove the remembered key. Unlock the OS credential store and retry."
                    .into(),
            ),
        }
    }
}

fn validate_key(key: &str) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("Enter an API key.".into());
    }
    if key.trim().chars().any(|c| c.is_control()) {
        return Err("The API key contains invalid characters.".into());
    }
    Ok(())
}

fn validate(key: &str, model: &str) -> Result<(), String> {
    validate_key(key)?;
    if model.trim().is_empty() {
        return Err("Enter a model ID.".into());
    }
    if !model
        .trim()
        .bytes()
        .all(|c| c.is_ascii_alphanumeric() || b"-_.:".contains(&c))
    {
        return Err(
            "Enter a model ID using letters, numbers, hyphens, dots, underscores, or colons."
                .into(),
        );
    }
    Ok(())
}

fn test_connection(provider: Provider, key: &str, model: &str) -> Result<(), String> {
    validate(key, model)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not initialize the connection.")?;
    let request = match provider {
        Provider::OpenAi => client
            .get(format!("https://api.openai.com/v1/models/{model}"))
            .bearer_auth(key),
        Provider::Gemini => client
            .get(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{model}"
            ))
            .header("x-goog-api-key", key),
        Provider::Grok => client
            .get(format!("https://api.x.ai/v1/models/{model}"))
            .bearer_auth(key),
    };
    let response = request
        .send()
        .map_err(|_| "Could not connect. Check your network and try again.")?;
    match response.status().as_u16() {
        200..=299 => Ok(()),
        401 | 403 => Err("Access denied. Check your API key and model permissions.".into()),
        404 => Err("Model not found. Check the model ID and your account access.".into()),
        429 => Err("Provider rate limit or quota reached. Try again later.".into()),
        code => Err(format!("Connection test failed (HTTP {code}).")),
    }
}

// Provider responses have different envelopes; normalize IDs for the picker and
// for the existing model-specific connection check.
fn parse_models(
    provider: Provider,
    value: &serde_json::Value,
) -> Result<(Vec<String>, Option<String>), String> {
    let field = if provider == Provider::Gemini {
        "models"
    } else {
        "data"
    };
    let rows = value
        .get(field)
        .and_then(|v| v.as_array())
        .ok_or("The provider returned an unexpected model list.")?;
    let mut models = Vec::new();
    for row in rows {
        let field = if provider == Provider::Gemini {
            "name"
        } else {
            "id"
        };
        let id = row
            .get(field)
            .and_then(|v| v.as_str())
            .ok_or("The provider returned an invalid model ID.")?;
        let id = if provider == Provider::Gemini {
            id.strip_prefix("models/").unwrap_or(id)
        } else {
            id
        };
        if validate("placeholder", id).is_ok() {
            models.push(id.to_owned());
        }
    }
    let next = match value.get("nextPageToken") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(token)) if token.is_empty() => None,
        Some(serde_json::Value::String(token)) => Some(token.clone()),
        _ => return Err("The provider returned an invalid model page.".into()),
    };
    Ok((models, next))
}

fn load_models(provider: Provider, key: &str) -> Result<Vec<String>, String> {
    validate_key(key)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not initialize the connection.")?;
    let mut models = Vec::new();
    let mut page = None;
    let mut seen = std::collections::HashSet::new();
    for _ in 0..100 {
        let mut request = match provider {
            Provider::OpenAi => client
                .get("https://api.openai.com/v1/models")
                .bearer_auth(key),
            Provider::Grok => client.get("https://api.x.ai/v1/models").bearer_auth(key),
            Provider::Gemini => client
                .get("https://generativelanguage.googleapis.com/v1beta/models")
                .header("x-goog-api-key", key)
                .query(&[("pageSize", "1000")]),
        };
        if let Some(token) = &page {
            request = request.query(&[("pageToken", token)]);
        }
        let response = request
            .send()
            .map_err(|_| "Could not load models. Check your network and try again.")?;
        match response.status().as_u16() {
            200..=299 => {}
            401 | 403 => {
                return Err("Access denied. Check your API key and model-list permissions.".into());
            }
            429 => return Err("Provider rate limit or quota reached. Try again later.".into()),
            code => return Err(format!("Could not load models (HTTP {code}).")),
        }
        let bytes = response
            .bytes()
            .map_err(|_| "Could not read the model list.")?;
        let value = serde_json::from_slice(&bytes)
            .map_err(|_| "The provider returned an invalid model list.")?;
        let (ids, next) = parse_models(provider, &value)?;
        models.extend(ids);
        if provider != Provider::Gemini || next.is_none() {
            models.sort();
            models.dedup();
            return Ok(models);
        }
        let token = next.unwrap();
        if !seen.insert(token.clone()) {
            return Err("The provider repeated a model page. Try again.".into());
        }
        page = Some(token);
    }
    Err("The model list exceeded the page limit. Enter a model ID manually.".into())
}

impl Settings {
    pub fn button_label(&self) -> String {
        format!(
            "{} ▾",
            self.preferences.selected.map_or("AI setup", Provider::name)
        )
    }

    pub fn chat_title(&self) -> String {
        if let Some(provider) = self.preferences.selected {
            let prefs = &self.preferences.providers[provider.index()];
            if !prefs.model.is_empty()
                && (prefs.remember || !self.keys[provider.index()].is_empty())
            {
                return format!("AI commands ({} · {})", provider.name(), prefs.model);
            }
        }
        "AI commands (not connected)".into()
    }

    pub fn begin(&mut self) {
        self.draft = self.preferences.selected.unwrap_or_default();
        self.open = true;
    }

    fn configure(&mut self, provider: Provider) {
        let prefs = &self.preferences.providers[provider.index()];
        let mut key = self.keys[provider.index()].clone();
        let mut message = String::new();
        if key.is_empty() && prefs.remember {
            match provider.entry().and_then(|entry| entry.get_password()
                .map_err(|_| "Could not retrieve the saved key. Unlock your credential store or enter it again.".into())) {
                Ok(saved) => key = saved,
                Err(error) => message = error,
            }
        }
        self.configuration = Some(Configuration {
            provider,
            key,
            model: prefs.model.clone(),
            remember: prefs.remember,
            message,
            pending: None,
            models: Vec::new(),
        });
    }

    fn save_configuration(&mut self) -> Result<(), String> {
        let config = self.configuration.as_ref().ok_or("No provider selected.")?;
        validate(&config.key, &config.model)?;
        let path = preferences_path()?;
        // Fail before touching credentials if the settings folder cannot be created.
        std::fs::create_dir_all(path.parent().ok_or("Invalid settings path.")?)
            .map_err(|_| "Could not create the settings folder.")?;
        if config.remember || self.preferences.providers[config.provider.index()].remember {
            store_key(
                &config.provider.entry()?,
                config.key.trim(),
                config.remember,
            )?;
        }
        let mut preferences = self.preferences.clone();
        preferences.providers[config.provider.index()] = ProviderPreferences {
            model: config.model.trim().into(),
            remember: config.remember,
        };
        write_preferences(&preferences, &path)?;
        self.keys[config.provider.index()] = config.key.trim().into();
        self.preferences = preferences;
        Ok(())
    }

    pub fn dialog(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        let mut open = true;
        let mut close = false;
        let mut configure = None;
        egui::Window::new("AI setup")
            .id(egui::Id::new("ai_setup"))
            .order(egui::Order::Foreground)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_enabled_ui(self.configuration.is_none(), |ui| {
                    ui.label("AI provider");
                    egui::Grid::new("ai_providers").show(ui, |ui| {
                        for provider in Provider::ALL {
                            ui.radio_value(&mut self.draft, provider, provider.name());
                            if ui.button("Configure").clicked() {
                                configure = Some(provider);
                            }
                            ui.end_row();
                        }
                    });
                    if !self.message.is_empty() {
                        ui.label(&self.message);
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                        if ui.button("Use provider").clicked() {
                            let mut preferences = self.preferences.clone();
                            preferences.selected = Some(self.draft);
                            match preferences_path()
                                .and_then(|path| write_preferences(&preferences, &path))
                            {
                                Ok(()) => {
                                    self.preferences = preferences;
                                    close = true;
                                }
                                Err(error) => self.message = error,
                            }
                        }
                    });
                });
            });
        if let Some(provider) = configure {
            self.configure(provider);
        }
        self.open = open && !close;
        if !self.open {
            self.configuration = None;
            return;
        }
        self.configuration_dialog(ctx);
    }

    fn configuration_dialog(&mut self, ctx: &egui::Context) {
        let Some(config) = self.configuration.as_mut() else {
            return;
        };
        if let Some(pending) = &config.pending {
            let result = pending.lock().ok().and_then(|mut result| result.take());
            if let Some(result) = result {
                config.message = match result {
                    Ok(ProviderResult::Models(models)) => {
                        config.message = if models.is_empty() { "No models were returned for this key. Enter a model ID manually.".into() }
                            else { format!("Loaded {} models. Choose one below.", models.len()) };
                        config.models = models;
                        config.message.clone()
                    },
                    Ok(ProviderResult::Connection) => "Connection successful. Key and model access verified; generation was not tested.".into(),
                    Err(error) => error,
                };
                config.pending = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
        let mut cancel = false;
        let mut save = false;
        let modal = egui::Modal::new(egui::Id::new("ai_provider_configuration")).show(ctx, |ui| {
            ui.set_width(400.0);
            ui.horizontal(|ui| {
                ui.heading(format!("Configure {}", config.provider.name()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("×")
                        .on_hover_text("Cancel configuration")
                        .clicked()
                    {
                        cancel = true;
                    }
                });
            });
            ui.add_enabled_ui(config.pending.is_none(), |ui| {
                ui.label("API key");
                let key_changed = ui
                    .add(
                        egui::TextEdit::singleline(&mut config.key)
                            .password(true)
                            .desired_width(360.0),
                    )
                    .changed();
                ui.checkbox(&mut config.remember, "Remember key in OS credential store");
                ui.label("Otherwise the key is kept only for this session.");
                if key_changed {
                    config.models.clear();
                }
                if ui
                    .button(if config.models.is_empty() {
                        "Load models"
                    } else {
                        "Refresh models"
                    })
                    .clicked()
                {
                    match validate_key(&config.key) {
                        Err(error) => config.message = error,
                        Ok(()) => {
                            let result = Arc::new(Mutex::new(None));
                            config.pending = Some(result.clone());
                            config.message = "Loading models…".into();
                            let provider = config.provider;
                            let key = config.key.trim().to_owned();
                            let ctx = ctx.clone();
                            std::thread::spawn(move || {
                                let loaded =
                                    load_models(provider, &key).map(ProviderResult::Models);
                                if let Ok(mut result) = result.lock() {
                                    *result = Some(loaded);
                                }
                                ctx.request_repaint();
                            });
                        }
                    }
                }
                ui.label("Model");
                ui.add_enabled_ui(!config.models.is_empty(), |ui| {
                    egui::ComboBox::from_id_salt("available_ai_models")
                        .width(360.0)
                        .selected_text(if config.model.is_empty() {
                            "Choose a model"
                        } else {
                            &config.model
                        })
                        .show_ui(ui, |ui| {
                            for model in &config.models {
                                if ui
                                    .selectable_value(&mut config.model, model.clone(), model)
                                    .changed()
                                {
                                    config.message.clear();
                                }
                            }
                        });
                });
                ui.label("Or enter a model ID manually");
                let model_changed = ui
                    .add(
                        egui::TextEdit::singleline(&mut config.model)
                            .hint_text("Enter the model ID from your provider")
                            .desired_width(360.0),
                    )
                    .changed();
                if key_changed || model_changed {
                    config.message.clear();
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            config.pending.is_none(),
                            egui::Button::new("Test connection"),
                        )
                        .clicked()
                    {
                        match validate(&config.key, &config.model) {
                            Err(error) => config.message = error,
                            Ok(()) => {
                                let result = Arc::new(Mutex::new(None));
                                config.pending = Some(result.clone());
                                config.message = "Testing connection…".into();
                                let provider = config.provider;
                                let key = config.key.trim().to_owned();
                                let model = config.model.trim().to_owned();
                                let ctx = ctx.clone();
                                std::thread::spawn(move || {
                                    let tested = test_connection(provider, &key, &model)
                                        .map(|()| ProviderResult::Connection);
                                    if let Ok(mut result) = result.lock() {
                                        *result = Some(tested);
                                    }
                                    ctx.request_repaint();
                                });
                            }
                        }
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
                if ui
                    .add_enabled(config.pending.is_none(), egui::Button::new("Save"))
                    .clicked()
                {
                    save = true;
                }
            });
            if !config.message.is_empty() {
                ui.label(&config.message);
            }
        });
        if cancel || modal.should_close() {
            self.configuration = None;
        } else if save {
            match self.save_configuration() {
                Ok(()) => {
                    self.configuration = None;
                    self.message = "Provider configuration saved.".into();
                }
                Err(error) => self.configuration.as_mut().unwrap().message = error,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dialog_settings(pending: Option<TestResult>) -> Settings {
        Settings {
            preferences: Preferences::default(),
            keys: Default::default(),
            draft: Provider::OpenAi,
            open: true,
            message: String::new(),
            configuration: Some(Configuration {
                provider: Provider::OpenAi,
                key: String::new(),
                model: String::new(),
                remember: false,
                message: String::new(),
                pending,
                models: Vec::new(),
            }),
        }
    }

    fn click_dialog_button(settings: &mut Settings, label: &str) {
        let ctx = egui::Context::default();
        let frame = |events| egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 800.0),
            )),
            events,
            ..Default::default()
        };
        // Allow the modal to establish its layout and input layer.
        let _ = ctx.run(frame(vec![]), |ctx| settings.dialog(ctx));
        let output = ctx.run(frame(vec![]), |ctx| settings.dialog(ctx));
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    if text.galley.text() == label {
                        return Some(text.pos + text.galley.size() * 0.5);
                    }
                }
                None
            })
            .unwrap_or_else(|| panic!("Button {label} was not rendered"));
        let _ = ctx.run(
            frame(vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ]),
            |ctx| settings.dialog(ctx),
        );
        let _ = ctx.run(
            frame(vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }]),
            |ctx| settings.dialog(ctx),
        );
    }

    #[test]
    fn configuration_cancel_works_during_and_after_connection_testing() {
        for result in [None, Some(Ok(ProviderResult::Connection))] {
            let mut settings = dialog_settings(Some(Arc::new(Mutex::new(result))));
            click_dialog_button(&mut settings, "Cancel");
            assert!(settings.configuration.is_none());
            assert!(settings.preferences.selected.is_none());
            assert!(settings.keys.iter().all(String::is_empty));
        }
    }

    #[test]
    fn configuration_save_is_clickable_after_connection_testing() {
        let mut settings = dialog_settings(Some(Arc::new(Mutex::new(Some(Ok(
            ProviderResult::Connection,
        ))))));
        click_dialog_button(&mut settings, "Save");
        // An invalid draft stays open and displays validation instead of writing settings.
        assert_eq!(
            settings.configuration.as_ref().unwrap().message,
            "Enter an API key."
        );
    }

    #[test]
    fn model_lists_parse_provider_envelopes_and_gemini_pagination() {
        let value = serde_json::json!({"data": [{"id": "model-b"}, {"id": "model-a"}]});
        for provider in [Provider::OpenAi, Provider::Grok] {
            let (models, next) = parse_models(provider, &value).unwrap();
            assert_eq!(models, ["model-b", "model-a"]);
            assert!(next.is_none());
        }
        let value = serde_json::json!({
            "models": [{"name": "models/gemini-example"}], "nextPageToken": "page-2"
        });
        let (models, next) = parse_models(Provider::Gemini, &value).unwrap();
        assert_eq!(models, ["gemini-example"]);
        assert_eq!(next.as_deref(), Some("page-2"));
        assert!(parse_models(Provider::OpenAi, &serde_json::json!({"error": {}})).is_err());
        assert!(parse_models(Provider::Grok, &serde_json::json!({"data": [{}]})).is_err());
        assert!(
            parse_models(
                Provider::Gemini,
                &serde_json::json!({"models": [], "nextPageToken": 7})
            )
            .is_err()
        );
        let (models, _) = parse_models(
            Provider::OpenAi,
            &serde_json::json!({"data": [{"id": "../unsafe"}]}),
        )
        .unwrap();
        assert!(models.is_empty());
        assert!(load_models(Provider::OpenAi, "").is_err());
    }

    #[test]
    fn remembered_keys_can_be_replaced_and_removed_and_store_errors_are_reported() {
        let entry =
            keyring::Entry::new_with_credential(Box::new(keyring::mock::MockCredential::default()));
        store_key(&entry, "first-secret", true).unwrap();
        assert_eq!(entry.get_password().unwrap(), "first-secret");
        store_key(&entry, "replacement-secret", true).unwrap();
        assert_eq!(entry.get_password().unwrap(), "replacement-secret");
        let mock: &keyring::mock::MockCredential = entry.get_credential().downcast_ref().unwrap();
        mock.set_error(keyring::Error::Invalid("store".into(), "locked".into()));
        assert!(store_key(&entry, "another-secret", true).is_err());
        assert_eq!(entry.get_password().unwrap(), "replacement-secret");
        mock.set_error(keyring::Error::Invalid("store".into(), "locked".into()));
        assert!(store_key(&entry, "", false).is_err());
        store_key(&entry, "", false).unwrap();
        assert!(matches!(entry.get_password(), Err(keyring::Error::NoEntry)));
        store_key(&entry, "", false).unwrap();
    }

    #[test]
    fn preferences_round_trip_without_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings/ai.json");
        let mut prefs = Preferences {
            selected: Some(Provider::Gemini),
            ..Default::default()
        };
        prefs.providers[1] = ProviderPreferences {
            model: "example-model".into(),
            remember: true,
        };
        write_preferences(&prefs, &path).unwrap();
        let encoded = std::fs::read_to_string(&path).unwrap();
        assert!(!encoded.contains("key"));
        let loaded: Preferences = serde_json::from_str(&encoded).unwrap();
        assert!(loaded.selected == Some(Provider::Gemini));
        assert_eq!(loaded.providers[1].model, "example-model");
        assert!(loaded.providers[1].remember);
        assert!(!loaded.providers[0].remember);
        write_preferences(&Preferences::default(), &path).unwrap();
        let loaded: Preferences = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert!(loaded.selected.is_none());
    }

    #[test]
    fn invalid_credentials_and_model_paths_are_rejected_before_network_requests() {
        for (key, model) in [
            ("", "model"),
            ("token", ""),
            ("bad\nkey", "model"),
            ("token", "../models"),
            ("token", "model?key=secret"),
        ] {
            assert!(validate(key, model).is_err());
        }
        assert!(validate(" token ", " model-v1.2:latest ").is_ok());
        assert!(test_connection(Provider::OpenAi, "", "model").is_err());
    }
}
