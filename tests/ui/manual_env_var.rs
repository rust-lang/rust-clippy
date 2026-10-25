#![warn(clippy::manual_env_var)]
#![allow(unused)]
use std::collections::HashMap;
use std::env;
use std::ffi::{OsStr, OsString};

fn main() {
    let _: Option<String> = env::vars().find(|(k, v)| k == "FOO").map(|(k, v)| v);
    //~^ manual_env_var
    let _: Option<String> = env::vars().find(|(k, v)| k == "FOO").map(|e| e.1);
    //~^ manual_env_var
    let _: Option<String> = env::vars().find(|(k, v)| k == "FOO").map(|(k, _)| k);
    let _: Option<String> = env::vars().find(|(k, v)| k == "FOO").map(|e| e.0);

    let _: Option<(String, String)> = env::vars().find(|(k, v)| k == "FOO");

    let _: Option<String> = env::vars().find_map(|(k, v)| if k == "FOO" { Some(v) } else { None });
    //~^ manual_env_var
    let _: Option<String> = env::vars().find_map(|(k, v)| if k == "FOO" { Some(k) } else { None });

    let _: Option<String> = env::vars().find(|e| e.0 == "FOO").map(|(k, v)| v);
    //~^ manual_env_var
    let _: Option<String> = env::vars().find(|e| e.0 == "FOO").map(|e| e.1);
    //~^ manual_env_var
    let _: Option<String> = env::vars().find(|e| e.0 == "FOO").map(|(k, v)| k);
    let _: Option<String> = env::vars().find(|e| e.0 == "FOO").map(|e| e.0);

    let _: Option<(String, String)> = env::vars().find(|e| e.0 == "FOO");

    let _: Option<String> = env::vars().find_map(|e| if e.0 == "FOO" { Some(e.1) } else { None });
    //~^ manual_env_var
    let _: Option<String> = env::vars().find_map(|e| if e.0 == "FOO" { Some(e.0) } else { None });
    let _: Option<String> = env::vars().find_map(|e| if e.1 == "FOO" { Some(e.0) } else { None });

    let _: bool = env::vars().any(|(k, v)| k == "FOO");
    //~^ manual_env_var
    let _: bool = env::vars().any(|e| e.0 == "FOO");
    //~^ manual_env_var
    let _: bool = env::vars().any(|(k, v)| v == "FOO");
    let _: bool = env::vars().any(|e| e.1 == "FOO");

    let kv1: HashMap<String, String> = env::vars().collect();
    let _: Option<String> = kv1.get("FOO").cloned();
    //~^ manual_env_var
    let _: bool = kv1.contains_key("FOO");
    //~^ manual_env_var

    let mut kv2: HashMap<String, String> = env::vars().collect();
    kv2.insert("FOO".to_string(), "BAR".to_string());
    let _: Option<String> = kv2.get("FOO").cloned();
    let _: bool = kv2.contains_key("FOO");

    let _: Option<String> = env::vars().collect::<HashMap<String, String>>().get("FOO").cloned();
    //~^ manual_env_var
    let _: bool = env::vars().collect::<HashMap<String, String>>().contains_key("FOO");
    //~^ manual_env_var

    let _: Option<OsString> = env::vars_os().find(|(k, v)| k == "FOO").map(|(k, v)| v);
    //~^ manual_env_var
    let _: Option<OsString> = env::vars_os().find(|(k, v)| k == "FOO").map(|e| e.1);
    //~^ manual_env_var
    let _: Option<OsString> = env::vars_os().find(|(k, v)| k == "FOO").map(|(k, _)| k);
    let _: Option<OsString> = env::vars_os().find(|(k, v)| k == "FOO").map(|e| e.0);

    let _: Option<(OsString, OsString)> = env::vars_os().find(|(k, v)| k == "FOO");

    let _: Option<OsString> = env::vars_os().find_map(|(k, v)| if k == "FOO" { Some(v) } else { None });
    //~^ manual_env_var
    let _: Option<OsString> = env::vars_os().find_map(|(k, v)| if k == "FOO" { Some(k) } else { None });

    let _: Option<OsString> = env::vars_os().find(|e| e.0 == "FOO").map(|(k, v)| v);
    //~^ manual_env_var
    let _: Option<OsString> = env::vars_os().find(|e| e.0 == "FOO").map(|e| e.1);
    //~^ manual_env_var
    let _: Option<OsString> = env::vars_os().find(|e| e.0 == "FOO").map(|(k, v)| k);
    let _: Option<OsString> = env::vars_os().find(|e| e.0 == "FOO").map(|e| e.0);

    let _: Option<(OsString, OsString)> = env::vars_os().find(|e| e.0 == "FOO");

    let _: Option<OsString> = env::vars_os().find_map(|e| if e.0 == "FOO" { Some(e.1) } else { None });
    //~^ manual_env_var
    let _: Option<OsString> = env::vars_os().find_map(|e| if e.0 == "FOO" { Some(e.0) } else { None });
    let _: Option<OsString> = env::vars_os().find_map(|e| if e.1 == "FOO" { Some(e.0) } else { None });

    let _: bool = env::vars_os().any(|(k, v)| k == "FOO");
    //~^ manual_env_var
    let _: bool = env::vars_os().any(|e| e.0 == "FOO");
    //~^ manual_env_var
    let _: bool = env::vars_os().any(|(k, v)| v == "FOO");
    let _: bool = env::vars_os().any(|e| e.1 == "FOO");

    let kv1: HashMap<OsString, OsString> = env::vars_os().collect();
    let _: Option<OsString> = kv1.get(OsStr::new("FOO")).cloned();
    //~^ manual_env_var
    let _: bool = kv1.contains_key(OsStr::new("FOO"));
    //~^ manual_env_var

    let mut kv2: HashMap<OsString, OsString> = env::vars_os().collect();
    kv2.insert("FOO".to_string().into(), "BAR".to_string().into());
    let _: Option<OsString> = kv2.get(OsStr::new("FOO")).cloned();
    let _: bool = kv2.contains_key(OsStr::new("FOO"));

    let _: Option<OsString> = env::vars_os()
        //~^ manual_env_var
        .collect::<HashMap<OsString, OsString>>()
        .get(OsStr::new("FOO"))
        .cloned();
    let _: bool = env::vars_os()
        //~^ manual_env_var
        .collect::<HashMap<OsString, OsString>>()
        .contains_key(OsStr::new("FOO"));
}
