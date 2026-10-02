use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::thread::sleep;
use std::time::{Duration, Instant};

const FRAMES: [&str; 4] = [
    include_str!("frames/open.txt"),
    include_str!("frames/half.txt"),
    include_str!("frames/closed.txt"),
    include_str!("frames/brows.txt"),
];
const OPEN: usize = 0;
const HALF: usize = 1;
const CLOSED: usize = 2;
const BROWS: usize = 3;

const SEQUENCE: &[(usize, u64)] = &[
    (OPEN, 1800),
    (HALF, 50), (CLOSED, 110), (HALF, 50), (OPEN, 1600),
    (BROWS, 220), (OPEN, 180), (BROWS, 220), (OPEN, 2200),
    (HALF, 50), (CLOSED, 110), (HALF, 50), (OPEN, 180),
    (HALF, 50), (CLOSED, 110), (HALF, 50), (OPEN, 2000),
    (BROWS, 900), (OPEN, 1400),
];

const INFO_OFFSET: usize = 1;
const TICK: Duration = Duration::from_millis(25);

static STOP: AtomicBool = AtomicBool::new(false);
static RESIZED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_stop(_: libc::c_int) {
    STOP.store(true, Relaxed);
}

extern "C" fn on_resize(_: libc::c_int) {
    RESIZED.store(true, Relaxed);
}

fn terminal_size() -> Option<(usize, usize)> {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let ok = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) } == 0;
    (ok && ws.ws_row > 0 && ws.ws_col > 0).then(|| (ws.ws_row as usize, ws.ws_col as usize))
}

struct Screen<'a> {
    frames: Vec<Vec<String>>,
    info: &'a [String],
    width: usize,
    rows: usize,
    size: Option<(usize, usize)>,
}

impl Screen<'_> {
    fn art(&self, frame: usize, row: usize) -> &str {
        self.frames[frame].get(row).map_or("", String::as_str)
    }

    fn clip(&self, line: &str) -> String {
        line.chars().take(self.size.map_or(usize::MAX, |(_, c)| c)).collect()
    }

    fn full(&self, frame: usize) -> String {
        (0..self.rows)
            .map(|i| {
                let text = i.checked_sub(INFO_OFFSET).and_then(|j| self.info.get(j)).map_or("", String::as_str);
                let line = format!("{:<w$}   {}", self.art(frame, i), text, w = self.width);
                format!("{}\x1B[K\n", self.clip(&line).trim_end())
            })
            .collect()
    }

    fn art_only(&self, frame: usize) -> String {
        let body: String = (0..self.rows)
            .map(|i| format!("\r{}\n", self.clip(&format!("{:<w$}", self.art(frame, i), w = self.width))))
            .collect();
        format!("\x1B[{}A{}", self.rows, body)
    }

    fn fits(&self) -> bool {
        self.size.map_or(false, |(h, _)| self.rows < h)
    }
}

fn emit(out: &mut impl Write, text: &str) -> bool {
    out.write_all(text.as_bytes()).and_then(|_| out.flush()).is_ok()
}

pub fn run(info: &[String]) {
    let width = FRAMES[OPEN].lines().map(|l| l.chars().count()).max().unwrap_or(0);
    let frames: Vec<Vec<String>> = FRAMES.iter().map(|f| f.lines().map(str::to_string).collect()).collect();
    let mut screen = Screen {
        rows: frames[OPEN].len().max(info.len() + INFO_OFFSET),
        frames,
        info,
        width,
        size: terminal_size(),
    };

    let mut out = io::stdout().lock();
    if !emit(&mut out, &screen.full(OPEN)) || !out.is_terminal() || std::env::var_os("MILADYFETCH_NO_ANIM").is_some() {
        return;
    }

    unsafe {
        libc::signal(libc::SIGINT, on_stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, on_stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGWINCH, on_resize as *const () as libc::sighandler_t);
    }

    emit(&mut out, "\x1B[?25l");
    'outer: for &(frame, ms) in SEQUENCE.iter().cycle() {
        if screen.fits() && !emit(&mut out, &screen.art_only(frame)) {
            break;
        }
        let end = Instant::now() + Duration::from_millis(ms);
        while let Some(left) = end.checked_duration_since(Instant::now()) {
            if STOP.load(Relaxed) {
                break 'outer;
            }
            if RESIZED.swap(false, Relaxed) {
                screen.size = terminal_size();
                if !emit(&mut out, &format!("\x1B[H\x1B[2J{}", screen.full(frame))) {
                    break 'outer;
                }
            }
            sleep(left.min(TICK));
        }
    }
    if screen.fits() {
        emit(&mut out, &screen.art_only(OPEN));
    }
    emit(&mut out, "\r\x1B[K\x1B[?25h");
}
