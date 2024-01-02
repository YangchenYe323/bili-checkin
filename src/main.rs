use std::{path::Path, time::Duration};

use bili_api_rs::apis::live::user::{GetMedalForUserResponse, MedalItem};
use clap::Parser;
use cli::Cli;

mod cli;

fn main() {
    let Cli { cookie, msg } = Cli::parse();
    let cookie = Cookie::read_from_file(cookie.as_path());
    let cookie_str = format!("SESSDATA={}", cookie.sessdata);
    let medals = get_all_unlighted_medals(&cookie_str);
    light_medals(&cookie, &cookie_str, &msg, &medals);
}

#[derive(Debug)]
struct Cookie {
    pub sessdata: String,
    pub bili_jct: String,
}

impl Cookie {
    fn read_from_file(path: impl AsRef<Path>) -> Self {
        let file = std::fs::File::open(path.as_ref()).expect("Cannot open cookie file");
        let value: serde_json::Value =
            serde_json::from_reader(file).expect("Malformed cookie file");
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

        Cookie { sessdata, bili_jct }
    }
}

fn get_all_unlighted_medals(cookie_str: &str) -> Vec<MedalItem> {
    let mut medals = vec![];
    let agent = ureq::Agent::new();
    let mut cur_page = 1;
    let mut total_page = 10;
    while cur_page < total_page {
        let response =
            bili_api_rs::apis::live::user::get_medal_for_user(&agent, 10, cur_page, cookie_str)
                .expect("Failed to fetch user medal");
        match response {
            GetMedalForUserResponse::Success {
                code: _,
                data,
                ttl: _,
            } => {
                total_page = data.page_info.total_page;
                cur_page = data.page_info.cur_page + 1;
                for item in data.items {
                    if item.is_lighted == 0 {
                        medals.push(item);
                    }
                }
            }

            GetMedalForUserResponse::Failure {
                code: _,
                message,
                ttl: _,
            } => {
                panic!("请求用户粉丝牌失败: {}", message)
            }
        }
    }
    medals
}

fn light_medals(cookie: &Cookie, cookie_str: &str, msg: &str, medals: &[MedalItem]) {
    let agent = ureq::Agent::new();
    for medal in medals {
        println!("正在点亮灯牌 [{}]...", &medal.medal_name);
        let room = medal.roomid;
        match bili_api_rs::apis::live::msg::send_live_message(
            &agent,
            room,
            msg,
            0xffffff,
            25,
            1,
            1,
            &cookie.bili_jct,
            cookie_str,
        ) {
            Ok(r) => (),
            Err(e) => {
                println!("点亮 [{}] 失败: {}", &medal.medal_name, e);
            }
        }

        std::thread::sleep(Duration::from_secs(2));
    }
}
