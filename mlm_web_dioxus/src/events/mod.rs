mod components;
mod server_fns;
mod types;

pub use components::{EventContent, EventListItem, EventsPage};
pub use server_fns::get_events_data;
pub use types::{EventData, EventWithTorrentData};

// Re-export the SSE trigger for backward compatibility
pub use crate::sse::EVENTS_UPDATE_TRIGGER;
pub use crate::sse::trigger_events_update;
