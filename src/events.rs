use chrono::Local;
use shiorust::message::{parts::HeaderName, traits::*, Request};

use std::{path::PathBuf, str::FromStr};

use crate::response::PluginResponse;
use crate::RPCCLIENT;

use encoding_rs::{SHIFT_JIS, UTF_8};
use std::collections::HashMap;
use std::fs;

pub type GhostDescript = HashMap<String, String>;

pub fn load_descript(file_path: String) -> GhostDescript {
    let mut descript: GhostDescript = HashMap::new();
    let buffer = fs::read(file_path).unwrap();
    let mut result = SHIFT_JIS.decode(&buffer).0;

    // TODO: more smart way to detect charset
    if result
        .clone()
        .into_owned()
        .as_str()
        .find("charset,UTF-8")
        .is_some()
    {
        result = UTF_8.decode(&buffer).0;
    }

    let input_text = result.into_owned();
    for line in input_text.lines() {
        if line.match_indices(",").count() != 1 {
            continue;
        }
        let mut iter = line.split(",");
        let key = iter.next().unwrap().to_string();
        let value = iter.next().unwrap().to_string();
        descript.insert(key, value);
    }
    descript
}

pub fn handle_request(req: &Request) -> PluginResponse {
    let id = req
        .headers
        .get(&HeaderName::from_str("ID").unwrap())
        .unwrap()
        .as_str();

    debug!("event: {}", id);

    let event = match id {
        "version" => version,
        "OnGhostBoot" => on_ghost_boot,
        "OnGhostExit" => on_ghost_exit,
        "OnMenuExec" => on_exec_menu,
        _ => return PluginResponse::new_nocontent(),
    };

    event(req)
}

fn version(_req: &Request) -> PluginResponse {
    let mut r = PluginResponse::new();
    r.response.headers.insert(
        HeaderName::from("Script"),
        String::from(env!("CARGO_PKG_VERSION")),
    );
    r
}

fn on_ghost_boot(req: &Request) -> PluginResponse {
    let reference1 = match req
        .headers
        .get(&HeaderName::from_str("Reference1").unwrap())
    {
        Some(reference1) => reference1,
        None => return PluginResponse::new_nocontent(),
    };

    let reference4 = match req
        .headers
        .get(&HeaderName::from_str("Reference4").unwrap())
    {
        Some(reference4) => reference4,
        None => return PluginResponse::new_nocontent(),
    };

    let ghost_name = reference1.as_str();
    let descript_path = {
        let mut p = PathBuf::from(reference4);
        p.push("ghost");
        p.push("master");
        p.push("descript.txt");
        p
    };

    let descript = load_descript(descript_path.into_os_string().into_string().unwrap());
    let craftmanurl = descript
        .get("craftmanurl")
        .unwrap_or(&String::from(""))
        .to_string();

    debug!("queued ghost's descript: {:?}", descript);
    debug!("queued ghost_name: {}", ghost_name);
    unsafe {
        RPCCLIENT.add_ghost(
            ghost_name.to_string(),
            Local::now().timestamp(),
            craftmanurl,
        );
    }

    PluginResponse::new_nocontent()
}

fn on_ghost_exit(req: &Request) -> PluginResponse {
    let reference1 = req
        .headers
        .get(&HeaderName::from_str("Reference1").unwrap());

    let ghost_name = match reference1 {
        Some(reference1) => reference1,
        None => return PluginResponse::new_nocontent(),
    };

    debug!("ghost_name: {}", ghost_name);
    unsafe {
        RPCCLIENT.remove_ghost(ghost_name.to_string());
    }

    PluginResponse::new_nocontent()
}

fn on_exec_menu(_req: &Request) -> PluginResponse {
    let mut r = PluginResponse::new();

    let s = format!(
        "\\_qukaing v{}\\n\\n\\q[✕,]",
        String::from(env!("CARGO_PKG_VERSION"))
    );

    r.response.headers.insert(HeaderName::from("Script"), s);
    r
}
