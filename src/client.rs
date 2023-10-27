use core::sync::atomic::{AtomicBool, Ordering};
use discord_rich_presence::{
    activity::{Activity, Button, Timestamps},
    DiscordIpc, DiscordIpcClient,
};
use std::sync::{Arc, Condvar, Mutex};

const TOKEN: &str = "1033946714102562826";
const BUTTON_LABEL: &str = "Craftman URL";

pub struct RpcClient {
    running: Arc<AtomicBool>,
    state: Arc<(Mutex<bool>, Condvar)>,
    thread: Option<std::thread::JoinHandle<()>>,
    ghost_list: Arc<Mutex<Vec<(String, i64, String)>>>,
}

impl RpcClient {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            state: Arc::new((Mutex::new(false), Condvar::new())),
            thread: None,
            ghost_list: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn start(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            debug!("before restart, stopping thread");
            self.close();
        }

        let state_clone = Arc::clone(&self.state);
        let running_clone = Arc::clone(&self.running);
        let list_clone = Arc::clone(&self.ghost_list);
        debug!("Starting Discord RPC thread");
        self.thread = Some(std::thread::spawn(move || {
            let mut client = DiscordIpcClient::new(TOKEN).unwrap();
            loop {
                match client.connect() {
                    Ok(_) => {
                        debug!("Connected to Discord RPC");
                        break;
                    }
                    Err(e) => {
                        debug!("Failed to connect to Discord RPC: {}", e);
                        for _ in 0..10 {
                            if !running_clone.load(Ordering::Relaxed) {
                                return;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }
                }
            }
            running_clone.store(true, Ordering::Relaxed);
            let (lock_c, cvar_c) = &*state_clone;
            let mut update = lock_c.lock().unwrap();
            loop {
                // Wait for the main thread to signal us to update
                update = cvar_c.wait_while(update, |u| !*u).unwrap();

                // Check if we should stop
                if !running_clone.load(Ordering::Relaxed) {
                    break;
                }

                // Update the presence
                let mut ghost_list = list_clone.lock().unwrap();
                // sort by timestamp
                ghost_list.sort_by(|a, b| a.1.cmp(&b.1));
                match ghost_list.last() {
                    Some((name, timestamp, craftmanurl)) => {
                        let mut ghost_name = String::from(name);
                        if ghost_name.chars().count() < 2 {
                            // state should be more than 1 char?
                            ghost_name.push_str(" ");
                        }
                        debug!("triggered update: {} {}", ghost_name, timestamp);
                        let mut activity = Activity::new()
                            .state(&ghost_name)
                            .timestamps(Timestamps::new().start(*timestamp));
                        if craftmanurl != "" {
                            activity =
                                activity.buttons(vec![Button::new(BUTTON_LABEL, craftmanurl)]);
                        }
                        match client.set_activity(activity) {
                            Ok(_) => {
                                debug!("update successful");
                            }
                            Err(e) => {
                                debug!("update failed: {}", e);
                            }
                        }
                    }
                    None => {
                        debug!("update was signalled but no activity was queued");
                        client.clear_activity().unwrap();
                    }
                };
                *update = false;
            }
            debug!("Discord thread stopped");
            client.close().unwrap();
        }));
    }

    pub fn close(&mut self) {
        if !self.running.load(Ordering::Relaxed) {
            return;
        }

        if let Some(t) = self.thread.take() {
            self.running.store(false, Ordering::Relaxed);
            let (lock, cvar) = &*self.state;
            *lock.lock().unwrap() = true;
            drop(lock);
            debug!("Signalling discord thread to stop");
            cvar.notify_all();
            if let Err(e) = t.join() {
                error!("Failed to join discord thread: {:?}", e);
            }
            self.thread = None;
        };
    }

    pub fn add_ghost(&mut self, ghost: String, start: i64, craftmanurl: String) {
        let list_clone = self.ghost_list.clone();
        let mut ghost_list = list_clone.lock().unwrap();
        ghost_list.push((ghost, start, craftmanurl));
        self.update();
    }

    pub fn remove_ghost(&mut self, ghost: String) {
        let list_clone = self.ghost_list.clone();
        let mut ghost_list = list_clone.lock().unwrap();
        match ghost_list.iter().position(|(g, _, _)| g == &ghost) {
            Some(i) => {
                ghost_list.swap_remove(i);
            }
            None => {
                warn!("Tried to remove ghost that doesn't exist");
            }
        }
        self.update();
    }

    pub fn update(&mut self) {
        debug!("signalling discord thread to update");
        let (lock, cvar) = &*self.state;
        *lock.lock().unwrap() = true;
        cvar.notify_all();
    }
}
