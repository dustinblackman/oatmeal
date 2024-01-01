use std::path;

use anyhow::bail;
use anyhow::Result;
use chrono::DateTime;
use chrono::Local;
use chrono::SecondsFormat;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::configuration::Config;
use crate::configuration::ConfigKey;
use crate::domain::models::Author;
use crate::domain::models::EditorContext;
use crate::domain::models::Message;
use crate::domain::models::Session;
use crate::domain::models::State;

pub struct Sessions {
    pub cache_dir: path::PathBuf,
}

impl Default for Sessions {
    fn default() -> Sessions {
        let cache_dir = dirs::cache_dir().unwrap().join("oatmeal/sessions");

        return Sessions::new(cache_dir);
    }
}

impl Sessions {
    pub fn new(cache_dir: path::PathBuf) -> Sessions {
        return Sessions { cache_dir };
    }

    pub fn create_id() -> String {
        return Uuid::new_v4()
            .to_string()
            .split('-')
            .enumerate()
            .filter_map(|(idx, str)| {
                if idx > 1 {
                    return None;
                }
                return Some(str);
            })
            .collect::<Vec<&str>>()
            .join("-");
    }

    fn get_file_path(&self, id: &str) -> path::PathBuf {
        return self.cache_dir.join(format!("{id}.yaml"));
    }

    /// Returns a list of sessions, but with only the first author message and
    /// context removed to save on memory.
    pub async fn list(&self) -> Result<Vec<Session>> {
        let mut sessions: Vec<Session> = vec![];
        if !self.cache_dir.exists() {
            return Ok(sessions);
        }

        let mut dir = fs::read_dir(&self.cache_dir).await?;
        while let Some(file) = dir.next_entry().await? {
            let payload = fs::read_to_string(file.path()).await?;
            let mut session: Session = serde_yaml::from_str(&payload)?;
            let author_messages = session
                .state
                .messages
                .iter()
                .filter(|e| return e.author == Author::User)
                .collect::<Vec<&Message>>();
            if !author_messages.is_empty() {
                session.state.messages = vec![author_messages[0].clone()];
            } else {
                session.state.messages = vec![];
            }

            session.state.backend_context = "".to_string();
            sessions.push(session);
        }

        sessions.sort_by_cached_key(|session| {
            return DateTime::parse_from_rfc3339(&session.timestamp).unwrap();
        });

        return Ok(sessions);
    }

    pub async fn load(&self, id: &str) -> Result<Session> {
        let file_path = self.get_file_path(id);
        if !file_path.exists() {
            bail!(format!("No session found for id {id}"));
        }

        let payload = fs::read_to_string(file_path).await?;
        let session: Session = serde_yaml::from_str(&payload)?;

        return Ok(session);
    }

    pub async fn save(
        &self,
        id: &str,
        backend_context: &str,
        editor_context: &Option<EditorContext>,
        messages: &[Message],
    ) -> Result<()> {
        let mut state = State {
            // TODO drop pulling this in from config.
            backend_name: Config::get(ConfigKey::Backend),
            backend_model: Config::get(ConfigKey::Model),
            backend_context: backend_context.to_string(),
            editor_language: "".to_string(),
            messages: messages.to_vec(),
        };

        if let Some(context) = editor_context {
            state.editor_language = context.language.to_string();
        }

        let session = Session {
            id: id.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: Local::now().to_rfc3339_opts(SecondsFormat::Secs, false),
            state,
        };

        let payload = serde_yaml::to_string(&session)?;

        if !self.cache_dir.exists() {
            fs::create_dir_all(&self.cache_dir).await?;
        }

        let mut file = fs::File::create(self.get_file_path(id)).await?;
        file.write_all(payload.as_bytes()).await?;

        return Ok(());
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let file_path = self.get_file_path(id);
        if !file_path.exists() {
            return Ok(());
        }

        fs::remove_file(file_path).await?;
        return Ok(());
    }

    pub async fn delete_all(&self) -> Result<()> {
        if !self.cache_dir.exists() {
            return Ok(());
        }

        fs::remove_dir_all(&self.cache_dir).await?;
        return Ok(());
    }
}
