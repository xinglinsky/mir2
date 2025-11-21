use std::fmt;
use std::sync::{Arc, Mutex};

use tracing::{Event, Level, Subscriber};
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, Layer};

const MAX_LOG_ENTRIES: usize = 512;

#[derive(Clone, Default)]
pub struct SharedLogs {
    logs: Arc<Mutex<Vec<String>>>,
    debug_logs: Arc<Mutex<Vec<String>>>,
    chat_logs: Arc<Mutex<Vec<String>>>,
}

impl SharedLogs {
    pub fn new() -> Self {
        SharedLogs::default()
    }

    pub fn snapshot(&self) -> (Vec<String>, Vec<String>, Vec<String>) {
        fn clone_vec(arc: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
            match arc.lock() {
                Ok(guard) => guard.clone(),
                Err(poisoned) => poisoned.into_inner().clone(),
            }
        }

        (
            clone_vec(&self.logs),
            clone_vec(&self.debug_logs),
            clone_vec(&self.chat_logs),
        )
    }

    fn push_ring(arc: &Arc<Mutex<Vec<String>>>, line: String) {
        if line.is_empty() {
            return;
        }
        if let Ok(mut buf) = arc.lock() {
            buf.push(line);
            if buf.len() > MAX_LOG_ENTRIES {
                let overflow = buf.len() - MAX_LOG_ENTRIES;
                buf.drain(0..overflow);
            }
        }
    }
}

pub struct LogBufferLayer {
    shared: SharedLogs,
}

impl LogBufferLayer {
    pub fn new(shared: SharedLogs) -> Self {
        LogBufferLayer { shared }
    }

    #[allow(dead_code)]
    pub fn shared_logs(&self) -> SharedLogs {
        self.shared.clone()
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" && self.message.is_empty() {
            self.message.push_str(&format!("{:?}", value));
        }
    }
}

impl<S> Layer<S> for LogBufferLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let level = *meta.level();
        let target = meta.target();

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let message = visitor.message;

        let mut line = format!("[{}] {}", level, target);
        if !message.is_empty() {
            line.push_str(": ");
            line.push_str(&message);
        }

        if target == "chat" {
            SharedLogs::push_ring(&self.shared.chat_logs, line);
        } else {
            match level {
                Level::DEBUG | Level::TRACE => {
                    SharedLogs::push_ring(&self.shared.debug_logs, line);
                }
                _ => {
                    SharedLogs::push_ring(&self.shared.logs, line);
                }
            }
        }
    }
}
