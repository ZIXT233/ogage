extern crate evdev_rs as evdev;
extern crate mio;

use evdev::*;
use evdev::enums::*;
use std::io;
use std::fs::File;
use std::path::Path;
use std::process::Command;
use std::os::unix::io::AsRawFd;
use mio::{Poll,Events,Token,Interest};
use mio::unix::SourceFd;

// ---------------- 新增：最小配置支持 ----------------
use std::collections::HashMap;

struct Keys {
    hotkey:         EventCode,
    bright_up:      EventCode,
    bright_down:    EventCode,
    vol_up:         EventCode,
    vol_down:       EventCode,
    vol_up2:        EventCode,
    vol_down2:      EventCode,
    bright_down2:   EventCode,
    bright_up2:     EventCode,
    volume_up:      EventCode,
    volume_down:    EventCode,
    mute:           EventCode,
}

fn default_keys() -> Keys {
    Keys {
        hotkey:       EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY5),
        bright_up:    EventCode::EV_KEY(EV_KEY::BTN_DPAD_UP),
        bright_down:  EventCode::EV_KEY(EV_KEY::BTN_DPAD_DOWN),
        vol_up:       EventCode::EV_KEY(EV_KEY::BTN_DPAD_RIGHT),
        vol_down:     EventCode::EV_KEY(EV_KEY::BTN_DPAD_LEFT),
        vol_up2:      EventCode::EV_KEY(EV_KEY::BTN_TR),
        vol_down2:    EventCode::EV_KEY(EV_KEY::BTN_TL),
        bright_down2: EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY3),
        bright_up2:   EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY4),
        volume_up:    EventCode::EV_KEY(EV_KEY::KEY_VOLUMEUP),
        volume_down:  EventCode::EV_KEY(EV_KEY::KEY_VOLUMEDOWN),
        mute:         EventCode::EV_KEY(EV_KEY::KEY_PLAYPAUSE),
    }
}

fn parse_ev_key(name: &str) -> Option<EventCode> {
    // 仅支持本程序实际用到的键名；够用且无依赖
    let k = match name {
        // DPAD
        "BTN_DPAD_UP"    => EV_KEY::BTN_DPAD_UP,
        "BTN_DPAD_DOWN"  => EV_KEY::BTN_DPAD_DOWN,
        "BTN_DPAD_LEFT"  => EV_KEY::BTN_DPAD_LEFT,
        "BTN_DPAD_RIGHT" => EV_KEY::BTN_DPAD_RIGHT,
        // 肩键 / 触发键
        "BTN_TL" => EV_KEY::BTN_TL,
        "BTN_TR" => EV_KEY::BTN_TR,
        "BTN_TL2" => EV_KEY::BTN_TL2,
        "BTN_TR2" => EV_KEY::BTN_TR2,
        // Trigger Happy
        "BTN_TRIGGER_HAPPY1" => EV_KEY::BTN_TRIGGER_HAPPY1,
        "BTN_TRIGGER_HAPPY2" => EV_KEY::BTN_TRIGGER_HAPPY2,
        "BTN_TRIGGER_HAPPY3" => EV_KEY::BTN_TRIGGER_HAPPY3,
        "BTN_TRIGGER_HAPPY4" => EV_KEY::BTN_TRIGGER_HAPPY4,
        "BTN_TRIGGER_HAPPY5" => EV_KEY::BTN_TRIGGER_HAPPY5,
        // 系统键
        "KEY_VOLUMEUP"   => EV_KEY::KEY_VOLUMEUP,
        "KEY_VOLUMEDOWN" => EV_KEY::KEY_VOLUMEDOWN,
        "KEY_PLAYPAUSE"  => EV_KEY::KEY_PLAYPAUSE,
        "KEY_POWER"      => EV_KEY::KEY_POWER,
        // 常见选择/开始（可当热键）
        "BTN_SELECT"     => EV_KEY::BTN_SELECT,
        "BTN_START"      => EV_KEY::BTN_START,
        _ => return None,
    };
    Some(EventCode::EV_KEY(k))
}

fn load_keys_from_conf(path: &str) -> Keys {
    let mut keys = default_keys();
    let data = std::fs::read_to_string(path).unwrap_or_default();
    let mut map: HashMap<String,String> = HashMap::new();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    // 小工具：把 map 里的某键名（如 "HOTKEY"）解析为 EventCode 并赋值
    let mut set = |field: &str, dst: &mut EventCode| {
        if let Some(name) = map.get(field) {
            if let Some(code) = parse_ev_key(name) {
                *dst = code;
            } else {
                eprintln!("ogage.conf: unknown key name for {} = {}", field, name);
            }
        }
    };

    set("HOTKEY",        &mut keys.hotkey);
    set("BRIGHT_UP",     &mut keys.bright_up);
    set("BRIGHT_DOWN",   &mut keys.bright_down);
    set("VOL_UP",        &mut keys.vol_up);
    set("VOL_DOWN",      &mut keys.vol_down);
    set("VOL_UP2",       &mut keys.vol_up2);
    set("VOL_DOWN2",     &mut keys.vol_down2);
    set("BRIGHT_DOWN2",  &mut keys.bright_down2);
    set("BRIGHT_UP2",    &mut keys.bright_up2);
    set("VOLUME_UP",     &mut keys.volume_up);
    set("VOLUME_DOWN",   &mut keys.volume_down);
    set("MUTE",          &mut keys.mute);

    keys
}
// ---------------- 最小配置支持到此为止 ----------------

/*fn blink1() { ... }*/
/*fn blink2() { ... }*/

fn process_event(_dev: &Device, ev: &InputEvent, hotkey: bool, k: &Keys) {
    if hotkey && ev.value == 1 {
        if ev.event_code == k.bright_up || ev.event_code == k.bright_up2 {
            Command::new("brightnessctl").args(&["s","+2%"]).output().expect("Failed to execute brightnessctl");
        }
        else if ev.event_code == k.bright_down || ev.event_code == k.bright_down2 {
            Command::new("brightnessctl").args(&["-n","s","2%-"]).output().expect("Failed to execute brightnessctl");
        }
        else if ev.event_code == k.vol_up || ev.event_code == k.vol_up2 {
            Command::new("amixer").args(&["-q", "sset", "Playback", "1%+"]).output().expect("Failed to execute amixer");
        }
        else if ev.event_code == k.vol_down || ev.event_code == k.vol_down2 {
            Command::new("amixer").args(&["-q", "sset", "Playback", "1%-"]).output().expect("Failed to execute amixer");
        }
        else if ev.event_code == EventCode::EV_KEY(EV_KEY::KEY_POWER) && ev.value > 0 {
            Command::new("finish.sh").spawn().ok().expect("Failed to execute shutdown process");
        }
    }
    else if ev.event_code == EventCode::EV_SW(EV_SW::SW_HEADPHONE_INSERT) {
        let dest = match ev.value { 1 => "SPK", _ => "HP" };
        Command::new("amixer").args(&["-q", "sset", "'Playback Path'", dest]).output().expect("Failed to execute amixer");
    }
    else if ev.event_code == EventCode::EV_KEY(EV_KEY::KEY_POWER) && ev.value == 1 {
        Command::new("pause.sh").spawn().ok().expect("Failed to execute suspend process");
    }
    else if ev.event_code == k.volume_up  && ev.value > 0 {
        Command::new("amixer").args(&["-q", "sset", "Playback", "1%+"]).output().expect("Failed to execute amixer");
    }
    else if ev.event_code == k.volume_down  && ev.value > 0 {
        Command::new("amixer").args(&["-q", "sset", "Playback", "1%-"]).output().expect("Failed to execute amixer");
    }
    else if ev.event_code == k.mute && ev.value > 0 {
        Command::new("mute_toggle.sh").output().expect("Failed to execute amixer");
    }
}

fn process_event2(_dev: &Device, ev: &InputEvent, selectkey: bool) {
    if selectkey {
        if ev.event_code == EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY4) && ev.value == 1 {
            Command::new("speak_bat_life.sh").spawn().ok().expect("Failed to execute battery reading out loud");
        }
    }
}

fn main() -> io::Result<()> {
    // 读取配置（若文件不存在或格式不对，会用默认键位）
    let keys = load_keys_from_conf("/home/ark/ogage.conf");

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1);
    let mut devs: Vec<Device> = Vec::new();
    let mut hotkey = false;
    let mut selectkey = false;

    let mut i = 0;
    for s in ["/dev/input/event10", "/dev/input/event9", "/dev/input/event8", "/dev/input/event7", "/dev/input/event6", "/dev/input/event5", "/dev/input/event4", "/dev/input/event3", "/dev/input/event2", "/dev/input/event1", "/dev/input/event0"].iter() {
        if !Path::new(s).exists() {
            println!("Path {} doesn't exist", s);
            continue;
        }
        let fd = File::open(Path::new(s)).unwrap();
        let mut dev = Device::new().unwrap();
        poll.registry().register(&mut SourceFd(&fd.as_raw_fd()), Token(i), Interest::READABLE)?;
        dev.set_fd(fd)?;
        devs.push(dev);
        println!("Added {}", s);
        i += 1;
    }

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            let dev = &mut devs[event.token().0];
            while dev.has_event_pending() {
                let e = dev.next_event(evdev_rs::ReadFlag::NORMAL);
                match e {
                    Ok(kv) => {
                        let ev = &kv.1;
                        // 热键状态：用配置中的 hotkey
                        if ev.event_code == keys.hotkey {
                            hotkey = ev.value == 1;
                            // let grab = if hotkey { GrabMode::Grab } else { GrabMode::Ungrab };
                            // dev.grab(grab)?;
                        }
                        process_event(&dev, &ev, hotkey, &keys);

                        // selectkey 保持原逻辑（你若想也改成可配置，可仿照 hotkey）
                        if ev.event_code == EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY1) {
                            selectkey = ev.value == 1 || ev.value == 2;
                        }
                        process_event2(&dev, &ev, selectkey)
                    },
                    _ => ()
                }
            }
        }
    }
}
