use dioxus::prelude::*;

pub static STATS_UPDATE_TRIGGER: GlobalSignal<u32> = Signal::global(|| 0);
pub static EVENTS_UPDATE_TRIGGER: GlobalSignal<u32> = Signal::global(|| 0);
pub static SELECTED_UPDATE_TRIGGER: GlobalSignal<u32> = Signal::global(|| 0);
pub static ERRORS_UPDATE_TRIGGER: GlobalSignal<u32> = Signal::global(|| 0);
pub static QBIT_PROGRESS: GlobalSignal<Vec<(u64, u32)>> = Signal::global(Vec::new);

pub fn trigger_stats_update() {
    #[cfg(not(feature = "server"))]
    {
        *STATS_UPDATE_TRIGGER.write() += 1;
    }
}

pub fn trigger_events_update() {
    #[cfg(not(feature = "server"))]
    {
        *EVENTS_UPDATE_TRIGGER.write() += 1;
    }
}

pub fn trigger_selected_update() {
    #[cfg(not(feature = "server"))]
    {
        *SELECTED_UPDATE_TRIGGER.write() += 1;
    }
}

pub fn trigger_errors_update() {
    #[cfg(not(feature = "server"))]
    {
        *ERRORS_UPDATE_TRIGGER.write() += 1;
    }
}

pub fn update_qbit_progress(progress: Vec<(u64, u32)>) {
    #[cfg(not(feature = "server"))]
    {
        *QBIT_PROGRESS.write() = progress;
    }
}
