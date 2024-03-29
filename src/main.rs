use std::path::{Path, PathBuf};

use bili_api_rs::{
    apis::live::{info, msg, user},
    credential::Credential,
    Error,
};

// 多个打卡消息绕过可能的屏蔽词导致弹幕发送失败
const MSGS: [&str; 5] = ["OvO", "( •́ .̫ •̀ )", "Check", "你好", "打卡"];

fn main() {
    let cookie: PathBuf = std::env::args()
        .nth(1)
        .expect("usage: bili-check <cookie file path>")
        .into();
    let agent = bili_api_rs::Client::new();
    let cookie = read_cred_from_file(cookie.as_path());
    let medals = get_unlighted_medals(&agent, &cookie);
    println!("总共 {} 个未点亮粉丝牌: ", medals.len());
    light_medals(&agent, &cookie, &medals);
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

fn get_unlighted_medals(agent: &bili_api_rs::Client, cookie: &Credential) -> Vec<user::MedalItem> {
    let mut medals = vec![];
    let mut cur_page = 1;
    let mut total_page = 10;
    while cur_page <= total_page {
        match user::get_medal_for_user(agent, 10, cur_page, cookie) {
            Ok(user::GetMedalForUserResponse { data }) => {
                total_page = data.page_info.total_page;
                cur_page = data.page_info.cur_page + 1;
                medals.extend(data.items.into_iter().filter(|item| item.is_lighted == 0));
            }
            Err(e) => {
                panic!("请求用户粉丝牌失败: {}", e);
            }
        }
    }
    medals
}

fn light_medals(agent: &bili_api_rs::Client, cookie: &Credential, medals: &[user::MedalItem]) {
    for medal in medals {
        println!("[{}]...正在点亮...", &medal.medal_name);
        match send_message_check_success(agent, cookie, medal) {
            true => {
                println!("[{}]...☑️", &medal.medal_name);
            }
            false => {
                println!("[{}]...无法点亮", &medal.medal_name);
            }
        };
    }
}

fn send_message_check_success(
    agent: &bili_api_rs::Client,
    credential: &Credential,
    medal: &user::MedalItem,
) -> bool {
    // 有时候MedalItem里的roomid是short id，确保使用original id来发送弹幕
    let room_info = info::get_live_room_info(agent, medal.roomid).expect("无法获取直播间信息");
    let room_id = room_info.data.room_id;

    for msg in MSGS {
        let config = msg::LiveMessageConfig::with_roomid_and_msg(room_id, msg.to_string());
        match msg::send_live_message(agent, config, credential) {
            Ok(r) if !r.message.is_empty() => {
                // 我们的弹幕可能包含屏蔽词，尝试其他弹幕组合
            }
            Ok(_r) => {
                return true;
            }
            Err(e) => {
                match e {
                    Error::Api(err) if err.code() == -403 => {
                        // 如果当前错误是粉丝牌等级禁言并且我们的勋章等级>=禁言等级，佩戴勋章后重试
                        if let Some(level) = msg::get_guard_level_threshold(&err) {
                            if medal.level >= level {
                                match user::wear_medal(agent, medal.medal_id, credential) {
                                    Ok(r) => {
                                        println!("{:?}", r);
                                    }
                                    Err(e) => {
                                        println!("  无法佩戴勋章: {}", e);
                                        return false;
                                    }
                                }
                            }
                        }
                    }
                    e => {
                        println!("  发送弹幕失败: {}", e);
                        return false;
                    }
                }
            }
        }
    }

    false
}
