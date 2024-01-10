use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub struct Cli {
  // User credential stored in cookie.json file format. I used the format
  // created by [biliup-rs](https://github.com/biliup/biliup-rs)
  #[arg(long, help = "用户登陆cookie文件, 可使用`https://github.com/biliup/biliup-rs`工具创建")]
  pub cookie: PathBuf,
}