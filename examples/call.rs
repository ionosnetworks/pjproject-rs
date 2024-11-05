use std::{
    ffi::{CStr, CString},
    mem::MaybeUninit,
    os::raw::c_void,
    panic,
};

use clap::{Arg, Command};
use pjproject_sys as pj;

unsafe extern "C" fn on_call_media_state(pjsua_call_id: i32) {
    let ci = {
        let mut ci = std::mem::MaybeUninit::<pj::pjsua_call_info>::uninit();
        let status = unsafe { pj::pjsua_call_get_info(pjsua_call_id, ci.as_mut_ptr()) };
        if status != pj::pj_constants__PJ_SUCCESS as i32 {
            panic!("Error in pjsua_call_get_info(): {status}");
        }
        ci.assume_init()
    };

    let user_data = unsafe {
        let ud = pj::pjsua_call_get_user_data(pjsua_call_id);
        Box::from_raw(ud as *mut CallUserData)
    };

    let state = CStr::from_ptr(ci.state_text.ptr);
    println!("Call {pjsua_call_id} state={}", state.to_string_lossy());

    if ci.media_status == pj::pjsua_call_media_status_PJSUA_CALL_MEDIA_ACTIVE {
        match user_data.play_file_path.as_ref() {
            Some(p) => {
                let mut player_id = MaybeUninit::uninit();
                unsafe {
                    let status = pj::pjsua_player_create(
                        &pj::pj_str(p.as_ptr() as *mut _) as *const _,
                        pj::pjmedia_file_player_option_PJMEDIA_FILE_NO_LOOP,
                        player_id.as_mut_ptr(),
                    );
                    if status != pj::pj_constants__PJ_SUCCESS as i32 {
                        panic!("Error in pjsua_player_create(): {status}");
                    }
                }
                let status = pj::pjsua_conf_connect(
                    pj::pjsua_player_get_conf_port(player_id.assume_init()),
                    ci.conf_slot,
                );
                if status != pj::pj_constants__PJ_SUCCESS as i32 {
                    panic!("Error in pjsua_conf_connect(): {status}");
                }
            }
            None => {
                pj::pjsua_conf_connect(ci.conf_slot, 0);
                pj::pjsua_conf_connect(0, ci.conf_slot);
            }
        }
    }

    std::mem::forget(user_data);
}

unsafe extern "C" fn on_call_state(pjsua_call_id: i32, pjsip_event: *mut pj::pjsip_event) {
    let event = pjsip_event.as_mut().unwrap();
    println!("Call {pjsua_call_id} event.type_: {}", event.type_);
}

struct CallUserData {
    pub play_file_path: Option<CString>,
}

fn main() {
    let matches = Command::new("Sip Call")
        .arg(
            Arg::new("sip-user")
                .long("sip-user")
                .help("Client SIP username")
                .takes_value(true)
                .value_name("USERNAME")
                .default_value("bob")
                .env("SIP_USER"),
        )
        .arg(
            Arg::new("sip-domain")
                .long("sip-domain")
                .help("Client SIP domain")
                .takes_value(true)
                .value_name("URI")
                .default_value("example.com")
                .env("SIP_DOMAIN"),
        )
        .arg(
            Arg::new("sip-port")
                .long("sip-port")
                .help("Client SIP Port")
                .takes_value(true)
                .value_name("PORT")
                .default_value("5060")
                .value_parser(clap::value_parser!(u16))
                .env("PORT"),
        )
        .arg(
            Arg::new("call-uri")
                .long("call-uri")
                .help("SIP URI to call")
                .takes_value(true)
                .value_name("URI")
                .required(true)
                .env("CALL_URI"),
        )
        .arg(
            Arg::new("play-live")
                .long("play-live")
                .help("Communicate live with device microphone and speaker")
                .takes_value(false)
                .default_value_if("play-file", None, Some("true"))
                .env("PLAY_LIVE"),
        )
        .arg(
            Arg::new("play-file")
                .long("play-file")
                .help("File to play when call succeeds")
                .takes_value(true)
                .value_name("FILE")
                .conflicts_with("play-live")
                .env("PLAY_FILE"),
        )
        .get_matches();

    /* Create pjsua first! */
    let status = unsafe { pj::pjsua_create() };
    if status != pj::pj_constants__PJ_SUCCESS as i32 {
        panic!("Error in pjsua_create(): {status}");
    }

    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        unsafe { pj::pjsua_call_hangup_all() };
        unsafe { pj::pjsua_destroy() };
        std::process::exit(1);
    }));

    let mut ctrlc_count = 0;
    ctrlc::set_handler(move || {
        tracing::warn!("Received ctrl-c, hanging up all...");
        if ctrlc_count >= 3 {
            tracing::error!("Quitting");
            std::process::exit(1);
        }
        ctrlc_count += 1;
        unsafe { pj::pjsua_call_hangup_all() };
        let status = unsafe { pj::pjsua_destroy() };

        std::process::exit(status)
    })
    .expect("Error setting Ctrl-C handler");

    let uri = matches.get_one::<String>("call-uri").unwrap();
    let uri = CString::new(uri.as_str()).expect("expected uri to convert to CString");
    let status = unsafe { pj::pjsua_verify_url(uri.as_ptr()) };
    if status != pj::pj_constants__PJ_SUCCESS as i32 {
        panic!("Invalid url({uri:?}): {status}");
    }

    let mut cfg = std::mem::MaybeUninit::<pj::pjsua_config>::uninit();
    unsafe { pj::pjsua_config_default(cfg.as_mut_ptr()) };

    /* Init pjsua */
    {
        let mut cfg = {
            let mut cfg = std::mem::MaybeUninit::<pj::pjsua_config>::uninit();
            unsafe {
                pj::pjsua_config_default(cfg.as_mut_ptr());
                cfg.assume_init()
            }
        };
        cfg.cb.on_call_media_state = Some(on_call_media_state);
        cfg.cb.on_call_state = Some(on_call_state);

        let mut log_cfg = {
            let mut log_cfg = std::mem::MaybeUninit::<pj::pjsua_logging_config>::uninit();
            unsafe {
                pj::pjsua_logging_config_default(log_cfg.as_mut_ptr());
                log_cfg.assume_init()
            }
        };
        log_cfg.console_level = 4;

        let status = unsafe { pj::pjsua_init(&cfg, &log_cfg, std::ptr::null()) };
        if status != pj::pj_constants__PJ_SUCCESS as i32 {
            panic!("Error in pjsua_init(): {status}");
        }
    }

    /* Add UDP transport. */
    {
        let mut cfg = {
            let mut cfg = std::mem::MaybeUninit::<pj::pjsua_transport_config>::uninit();
            unsafe {
                pj::pjsua_transport_config_default(cfg.as_mut_ptr());
                cfg.assume_init()
            }
        };
        cfg.port = *matches.get_one::<u16>("sip-port").unwrap() as u32;
        let status = unsafe {
            pj::pjsua_transport_create(
                pj::pjsip_transport_type_e_PJSIP_TRANSPORT_UDP,
                &cfg,
                std::ptr::null_mut(),
            )
        };
        if status != pj::pj_constants__PJ_SUCCESS as i32 {
            panic!("Error creating transport: {status}");
        }
    }

    /* Initialization is done, now start pjsua */
    let status = unsafe { pj::pjsua_start() };
    if status != pj::pj_constants__PJ_SUCCESS as i32 {
        panic!("Error starting pjsua: {status}");
    }

    /* Register to SIP server by creating SIP account. */
    let sip_user = matches.get_one::<String>("sip-user").unwrap();
    let sip_domain = matches.get_one::<String>("sip-domain").unwrap();
    let cfg_id = CString::new(format!("sip:{sip_user}@{sip_domain}")).unwrap();
    let mut acc_cfg = std::mem::MaybeUninit::<pj::pjsua_acc_config>::uninit();
    let acc_cfg_ptr = unsafe {
        pj::pjsua_acc_config_default(acc_cfg.as_mut_ptr());
        let acc_cfg = acc_cfg.assume_init_mut();
        acc_cfg.id = pj::pj_str(cfg_id.as_ptr() as *mut i8);

        acc_cfg
    };
    let mut acc_id = std::mem::MaybeUninit::<pj::pjsua_acc_id>::uninit();
    let status = unsafe {
        pj::pjsua_acc_add(
            acc_cfg_ptr as *const _,
            pj::pj_constants__PJ_TRUE as _,
            acc_id.as_mut_ptr(),
        )
    };
    if status != pj::pj_constants__PJ_SUCCESS as i32 {
        panic!("Error adding account: {status}");
    }

    let call_user_data = Box::new(CallUserData {
        play_file_path: matches
            .get_one::<String>("play-file")
            .map(|s| CString::new(s.as_str()).expect("failed to parse play_ile as cstring")),
    });
    let status = unsafe {
        let uri = pj::pj_str(uri.as_ptr() as *mut i8);
        pj::pjsua_call_make_call(
            acc_id.assume_init(),
            &uri,
            std::ptr::null(),
            Box::into_raw(call_user_data) as *mut c_void,
            std::ptr::null(),
            std::ptr::null_mut(),
        )
    };

    if status != pj::pj_constants__PJ_SUCCESS as i32 {
        panic!("Error adding account: {status}");
    }

    for line in std::io::stdin().lines() {
        match line {
            Ok(line) => {
                let line = line.trim();
                if line == "q" {
                    break;
                } else if line == "h" {
                    println!("hanging up all");
                    unsafe { pj::pjsua_call_hangup_all() };
                }
            }
            Err(err) => panic!("Error reading line from stdin: {err}"),
        }
    }

    let status = unsafe { pj::pjsua_destroy() };
    if status != pj::pj_constants__PJ_SUCCESS as i32 {
        eprintln!("Error in pjsua_destroy(): {status}");
    }
}
