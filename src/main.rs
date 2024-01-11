use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use bili_api_rs::{
    apis::live::user::{GetMedalForUserResponse, MedalItem},
    credential::Credential,
};
// 多个打卡消息绕过可能的屏蔽词导致弹幕发送失败
const MSGS: [&str; 5] = ["打卡", "OvO", "( •́ .̫ •̀ )", "Check", "你好"];

fn main() {
    let cookie: PathBuf = std::env::args()
        .nth(1)
        .expect("usage: bili-check <cookie file path>")
        .into();
    let cookie = read_cred_from_file(cookie.as_path());
    let medals = get_unlighted_medals(&cookie);
    println!("总共 {} 个未点亮粉丝牌粉丝牌: ", medals.len());
    light_medals(&cookie, &medals);
}

/// Credential file json format:
/// {
///   cookie_info: {
///     cookies: [
///       {
///         name,
///         value
///       },
///       ...
///     ]
///   }
/// }
/// I used [biliup-rs](https://github.com/biliup/biliup-rs)'s login utility to generate the file.
fn read_cred_from_file(path: impl AsRef<Path>) -> Credential {
    let file = std::fs::File::open(path.as_ref()).expect("Cannot open cookie file");
    let value: serde_json::Value = serde_json::from_reader(file).expect("Malformed cookie file");
    let cookie_info = value.get("cookie_info").expect("Malformed cookie file");
    let cookies = cookie_info.get("cookies").expect("Malformed cookie file");
    let cookies = cookies.as_array().expect("Malformed cookie file");
    let mut sessdata = None;
    let mut bili_jct = None;
    for entry in cookies {
        let name = entry.get("name").expect("Malformed cookie file");
        let name = name.as_str().expect("Malformed cookie file");
        if name != "SESSDATA" && name != "bili_jct" {
            continue;
        }

        let key = if name == "SESSDATA" {
            &mut sessdata
        } else {
            &mut bili_jct
        };
        let value = entry.get("value").expect("Malformed cookie file");
        let value = value.as_str().expect("Malformed cookie file");
        *key = Some(value.to_string())
    }

    if sessdata.is_none() || bili_jct.is_none() {
        panic!("Malformed cookie file");
    }

    let sessdata = sessdata.unwrap();
    let bili_jct = bili_jct.unwrap();

    Credential::new(sessdata, bili_jct)
}

fn get_unlighted_medals(cookie: &Credential) -> Vec<MedalItem> {
    let mut medals = vec![];
    let agent = reqwest::blocking::Client::new();
    let mut cur_page = 1;
    let mut total_page = 10;
    while cur_page <= total_page {
        match bili_api_rs::apis::live::user::get_medal_for_user(&agent, 10, cur_page, cookie) {
            Ok(GetMedalForUserResponse::Success {
                code: _,
                data,
                ttl: _,
            }) => {
                total_page = data.page_info.total_page;
                cur_page = data.page_info.cur_page + 1;
                medals.extend(data.items.into_iter().filter(|item| item.is_lighted == 0));
            }
            Ok(GetMedalForUserResponse::Failure {
                code: _,
                message,
                ttl: _,
            }) => {
                panic!("请求用户粉丝牌失败: {}", message);
            }
            Err(e) => {
                panic!("请求用户粉丝牌失败: {}", e);
            }
        }
    }
    medals
}

fn light_medals(cookie: &Credential, medals: &[MedalItem]) {
    let agent = reqwest::blocking::Client::new();
    for medal in medals {
        println!("[{}]...正在点亮...", &medal.medal_name);
        let room = medal.roomid;
        if !send_message_check_success(&agent, cookie, room) {
            println!("[{}]...无法点亮", &medal.medal_name);
        } else {
            println!("[{}]...☑️", &medal.medal_name);
        }
        std::thread::sleep(Duration::from_millis(1000));
    }
}

fn send_message_check_success(
    agent: &reqwest::blocking::Client,
    credential: &Credential,
    room: i32,
) -> bool {
    use bili_api_rs::apis::live::msg::SendLiveMessageResponse;
    const COLOR: i32 = 0xffffff;
    const FONT_SIZE: i32 = 25;
    const MODE: i32 = 0;
    const BUBBLE: i32 = 0;

    for msg in MSGS {
        match bili_api_rs::apis::live::msg::send_live_message(
            agent, room, msg, COLOR, FONT_SIZE, MODE, BUBBLE, credential,
        ) {
            Ok(SendLiveMessageResponse::Success {
                code,
                message,
                data: _,
            }) => {
                if code != 0 {
                    println!("  {}", &message);
                    return false;
                }
                if message.is_empty() {
                    return true;
                }
                // 当前弹幕可能被屏蔽导致弹幕发送失败，尝试其他弹幕组合
            }
            Ok(SendLiveMessageResponse::Failure { code: _, message }) => {
                println!("  {}", &message);
                return false;
            }
            Err(e) => {
                println!("  {}", e);
                return false;
            }
        }
    }

    false
}
