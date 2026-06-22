use std::fmt;
use std::io;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io {
        context: String,
        source: io::Error,
    },
    InvalidInput(String),
    SourceNotFound(String),
    SourceInvalidLayout(String),
    AgentNotConfigured(String),
    AgentSkillsDirInvalid(String),
    CacheMarkerMissing(String),
    CacheMarkerMismatch(String),
    CommandUnavailable(String),
    CommandFailed {
        program: String,
        status: Option<i32>,
        stderr: String,
    },
    Json {
        context: String,
        source: serde_json::Error,
    },
}

impl Error {
    pub fn io(context: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { context, source } => write!(f, "{context}: {source}"),
            Error::InvalidInput(message) => write!(f, "{message}"),
            Error::SourceNotFound(path) => write!(f, "source root does not exist: {path}"),
            Error::SourceInvalidLayout(path) => {
                write!(f, "source root does not contain a skills directory: {path}")
            }
            Error::AgentNotConfigured(agent) => write!(f, "agent is not configured: {agent}"),
            Error::AgentSkillsDirInvalid(path) => write!(f, "agent skills dir is invalid: {path}"),
            Error::CacheMarkerMissing(path) => write!(f, "cache marker is missing: {path}"),
            Error::CacheMarkerMismatch(path) => {
                write!(f, "cache marker does not match config: {path}")
            }
            Error::CommandUnavailable(program) => {
                write!(f, "required command is unavailable: {program}")
            }
            Error::CommandFailed {
                program,
                status,
                stderr,
            } => {
                let status = status
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "terminated by signal".to_string());
                if stderr.trim().is_empty() {
                    write!(f, "{program} failed with status {status}")
                } else {
                    write!(
                        f,
                        "{program} failed with status {status}: {}",
                        stderr.trim()
                    )
                }
            }
            Error::Json { context, source } => write!(f, "{context}: {source}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Json { source, .. } => Some(source),
            _ => None,
        }
    }
}
