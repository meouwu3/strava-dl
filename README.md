# strava-dl

Rust native CLI. 构建后是独立二进制，不需要 Python 或 Node.js。

将 Strava 运动员页面中当前会话可访问的图片和视频下载到系统 `Downloads` 目录，并输出文件夹或 ZIP。

```bash
cargo install --path ~/i/strava-media-archive
strava-dl 135539874
strava-dl 135539874 --output zip

# short alias
sdl 135539874
```

默认输出为 `~/Downloads/strava-<athlete-id>/`，ZIP 模式输出 `~/Downloads/strava-<athlete-id>.zip`。每个归档包含 `manifest.json`。

Strava 对活动媒体和视频通常要求登录或相应隐私权限。可传入浏览器会话 Cookie 或 API token：

```bash
STRAVA_COOKIE='name=value; ...' strava-dl 135539874
STRAVA_ACCESS_TOKEN='...' strava-dl 135539874
```

也可以使用 `--cookie` 或 `--token`。CLI 不绕过 Strava 权限控制，只下载当前会话实际可访问的媒体。

开发验证：`cargo test`。
