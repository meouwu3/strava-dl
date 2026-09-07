use std::io::{self, IsTerminal, Write};

const WIDTH: usize = 20;

fn enabled() -> bool {
    !cfg!(test) && io::stderr().is_terminal()
}

fn bounce_bar(step: usize) -> String {
    let block = 4.min(WIDTH);
    let travel = WIDTH - block;
    let idx = step.saturating_sub(1);
    let pos = if travel == 0 {
        0
    } else {
        let cycle = travel * 2;
        let t = idx % cycle;
        if t <= travel {
            t
        } else {
            cycle - t
        }
    };
    let mut chars = vec![' '; WIDTH];
    for i in 0..block {
        chars[pos + i] = '=';
    }
    format!("[{}]", chars.into_iter().collect::<String>())
}

fn ratio_bar(current: usize, total: usize) -> String {
    let filled = current
        .saturating_mul(WIDTH)
        .checked_div(total)
        .unwrap_or(0)
        .min(WIDTH);
    let mut chars = vec![' '; WIDTH];
    for (i, slot) in chars.iter_mut().enumerate() {
        if i < filled {
            *slot = '=';
        } else if i == filled {
            *slot = '>';
        }
    }
    format!("[{}]", chars.into_iter().collect::<String>())
}

fn draw(text: &str) {
    eprint!("\r{text}\x1b[K");
    let _ = io::stderr().flush();
}

pub struct ScanBar {
    drawn: bool,
}

impl ScanBar {
    pub fn new() -> Self {
        Self { drawn: false }
    }

    pub fn set(&mut self, page: usize) {
        if !enabled() {
            return;
        }
        self.drawn = true;
        draw(&format!("扫描 {} {page}", bounce_bar(page)));
    }

    pub fn finish(self) {
        if self.drawn {
            eprintln!();
        }
    }
}

pub struct DownloadBar {
    total: usize,
    drawn: bool,
}

impl DownloadBar {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            drawn: false,
        }
    }

    pub fn set(&mut self, current: usize) {
        if !enabled() {
            return;
        }
        self.drawn = true;
        draw(&format!(
            "下载 {} {current}/{}",
            ratio_bar(current, self.total),
            self.total
        ));
    }

    pub fn finish(self) {
        if self.drawn {
            eprintln!();
        }
    }
}

pub fn complete() {
    eprintln!("下载完成");
}

pub fn fail() {
    eprint!("\r\x1b[K");
    eprintln!("下载失败");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounce_bar_moves_with_page() {
        assert_eq!(bounce_bar(1), format!("[{}{}]", "====", " ".repeat(16)));
        assert_eq!(
            bounce_bar(2),
            format!("[{}{}{}]", " ", "====", " ".repeat(15))
        );
    }

    #[test]
    fn ratio_bar_tracks_current_over_total() {
        assert_eq!(ratio_bar(0, 10), format!("[>{}]", " ".repeat(19)));
        assert_eq!(ratio_bar(10, 10), format!("[{}]", "=".repeat(20)));
        assert_eq!(
            ratio_bar(5, 10),
            format!("[{}>{}]", "=".repeat(10), " ".repeat(9))
        );
    }
}
