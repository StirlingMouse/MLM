use dioxus::prelude::*;

pub static STATS_UPDATE_TRIGGER: GlobalSignal<u32> = Signal::global(|| 0);
pub static EVENTS_UPDATE_TRIGGER: GlobalSignal<u32> = Signal::global(|| 0);

pub fn trigger_stats_update() {
    #[cfg(not(feature = "server"))]
    {
        let mut val = STATS_UPDATE_TRIGGER.write();
        *val += 1;
    }
}

pub fn trigger_events_update() {
    #[cfg(not(feature = "server"))]
    {
        let mut val = EVENTS_UPDATE_TRIGGER.write();
        *val += 1;
    }
}
