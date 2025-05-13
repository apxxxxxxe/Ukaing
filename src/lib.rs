mod client;
mod events;
mod request;
mod response;
mod variables;

use client::RpcClient;
use request::PluginRequest;

use std::fs::File;
use std::panic;
use std::path::PathBuf;

use once_cell::sync::Lazy;

use winapi::ctypes::c_long;
use winapi::shared::minwindef::{BOOL, FALSE, HGLOBAL, TRUE};

use shiori_hglobal::*;
use shiorust::message::Parser;

#[macro_use]
extern crate log;
extern crate simplelog;

use simplelog::*;

static mut RPCCLIENT: Lazy<RpcClient> = Lazy::new(RpcClient::new);

#[no_mangle]
pub extern "cdecl" fn load(h: HGLOBAL, len: c_long) -> BOOL {
    let v = GStr::capture(h, len as usize);
    let s: String;
    match v.to_utf8_str() {
        Ok(st) => {
            // UTF-8に変換
            s = st.to_string();
        }
        Err(e) => {
            eprintln!("Failed to convert HGLOBAL to UTF-8: {:?}", e);
            match v.to_ansi_str() {
                Ok(st) => {
                    // ANSIに変換
                    s = st.to_string_lossy().to_string();
                }
                Err(e) => {
                    eprintln!("Failed to convert HGLOBAL to ANSI: {:?}", e);
                    return FALSE;
                }
            }
        }
    };

    // Windows(UTF-16)を想定しPathBufでパスを作成
    let log_path = PathBuf::from(&s).join("ukaing.log");
    WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create(log_path).unwrap(),
    )
    .unwrap();

    panic::set_hook(Box::new(|panic_info| {
        debug!("{}", panic_info);
    }));

    debug!("load");
    unsafe { RPCCLIENT.start() };

    TRUE
}

#[no_mangle]
pub extern "cdecl" fn unload() -> BOOL {
    debug!("unload");
    unsafe { RPCCLIENT.close() };
    TRUE
}

#[no_mangle]
pub extern "cdecl" fn request(h: HGLOBAL, len: &mut c_long) -> HGLOBAL {
    // リクエストの取得
    let v = GStr::capture(h, *len as usize);

    let s = v.to_utf8_str().unwrap();

    let pr = PluginRequest::parse(s).unwrap();
    let r = pr.request;

    let response = events::handle_request(&r);

    let bytes = response.to_string().into_bytes();
    let response_gstr = GStr::clone_from_slice_nofree(&bytes);

    *len = response_gstr.len() as c_long;
    response_gstr.handle()
}

#[cfg(test)]
mod test {
    const TOKEN: &str = "1033946714102562826";
    use discord_rich_presence::{
        activity::{Activity, Button, Timestamps},
        DiscordIpc, DiscordIpcClient,
    };
    use std::time::Duration;

    #[test]
    fn test_client() {
        let mut client = DiscordIpcClient::new(TOKEN).unwrap();
        client.connect().unwrap();
        let activity = Activity::new()
            .state("t ")
            .timestamps(Timestamps::new().start(chrono::Local::now().timestamp()))
            .buttons(vec![Button::new("hoge", "https://example.com/")]);
        client.set_activity(activity).unwrap();
        std::thread::sleep(Duration::from_secs(10));
        client.close().unwrap();
    }
}
