//! `dino`: the niubash runner. Jump the cacti, duck the birds, watch the
//! world speed up. Chrome taught a generation this game; niubash teaches it
//! in a shell. Space/↑ jumps, ↓ ducks, r restarts after a crash, q quits at
//! any time.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, terminal,
};

const WIDTH: usize = 58;
const GROUND_Y: usize = 13;
const DINO_X: usize = 8;

const START_TICK_MS: u64 = 110;
const MIN_TICK_MS: u64 = 55;

const DINO_STAND: &[&str] = &["  _", "(°>", "/|", "/\\"];
const DINO_RUN_A: &[&str] = &["  _", "(°>", "/|", "/ \\"];
const DINO_RUN_B: &[&str] = &["  _", "(°>", "/|", " /"];
const DINO_DUCK: &[&str] = &["    _", "_(°>"];

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// `w` columns of cactus, 3 rows tall — jump it.
    Cactus { w: usize },
    /// One-row bird — duck it (or time an early jump).
    Bird,
}

struct Obstacle {
    kind: Kind,
    x: usize,
    /// Row of the obstacle's top cell.
    y: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Running,
    Over,
}

struct Game {
    /// Rows climbed by the dino; 0 = on the ground.
    jump_y: usize,
    vy: i32,
    ducking: bool,
    run_frame: bool,
    obstacles: Vec<Obstacle>,
    travel: usize,
    score: u32,
    best: u32,
    phase: Phase,
    next_spawn: usize,
}

/// Dino collision box in cell units, relative to the field origin.
fn dino_box(g: &Game) -> (usize, usize, usize, usize) {
    if g.ducking && g.jump_y == 0 {
        // (x, y, w, h) — ducking hugs the ground.
        (
            DINO_X,
            GROUND_Y - DINO_DUCK.len(),
            DINO_DUCK[0].len(),
            DINO_DUCK.len(),
        )
    } else {
        let h = DINO_STAND.len();
        (DINO_X, GROUND_Y - h - g.jump_y, DINO_STAND[0].len(), h)
    }
}

fn obstacle_box(o: &Obstacle) -> (usize, usize, usize, usize) {
    match o.kind {
        Kind::Cactus { w } => (o.x, o.y, w, 3),
        Kind::Bird => (o.x, o.y, 3, 1),
    }
}

fn overlaps(a: (usize, usize, usize, usize), b: (usize, usize, usize, usize)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
}

impl Game {
    fn new(best: u32) -> Self {
        Game {
            jump_y: 0,
            vy: 0,
            ducking: false,
            run_frame: false,
            obstacles: Vec::new(),
            travel: 0,
            score: 0,
            best,
            phase: Phase::Running,
            // First obstacle shows up after a calm stretch.
            next_spawn: WIDTH + rand_n(20),
        }
    }

    fn tick_ms(&self) -> u64 {
        let step = (self.score / 50) as u64 * 5;
        START_TICK_MS.saturating_sub(step).max(MIN_TICK_MS)
    }

    fn tick(&mut self) {
        if self.phase != Phase::Running {
            return;
        }
        self.travel += 1;
        self.score = (self.travel / 2) as u32;
        self.run_frame = !self.run_frame;

        // Jump physics: vy is rows per tick upward, gravity decays it.
        if self.jump_y > 0 || self.vy != 0 {
            let climbed = self.jump_y as i32 + self.vy;
            if climbed <= 0 {
                self.jump_y = 0;
                self.vy = 0;
            } else {
                self.jump_y = climbed as usize;
                self.vy -= 1;
            }
        }

        for o in &mut self.obstacles {
            o.x = o.x.saturating_sub(1);
        }
        self.obstacles.retain(|o| o.x < WIDTH);

        if self.travel >= self.next_spawn {
            let roll = rand_n(10);
            if self.score > 30 && roll < 3 {
                // Birds fly at head height (duck) or jump-apex height (run under
                // is not possible; duck or jump early).
                let high = rand_n(2) == 0;
                let y = if high { GROUND_Y - 6 } else { GROUND_Y - 3 };
                self.obstacles.push(Obstacle {
                    kind: Kind::Bird,
                    x: WIDTH + 2,
                    y,
                });
            } else {
                let w = 1 + rand_n(3);
                self.obstacles.push(Obstacle {
                    kind: Kind::Cactus { w },
                    x: WIDTH + 2,
                    y: GROUND_Y - 3,
                });
            }
            self.next_spawn = self.travel + WIDTH / 2 + 6 + rand_n(18);
        }

        let dino = dino_box(self);
        if self
            .obstacles
            .iter()
            .any(|o| overlaps(dino, obstacle_box(o)))
        {
            self.phase = Phase::Over;
            self.best = self.best.max(self.score);
        }
    }

    fn jump(&mut self) {
        if self.phase == Phase::Running && self.jump_y == 0 {
            self.vy = 3;
            self.ducking = false;
        }
    }

    fn draw(&self, stdout: &mut io::Stdout) -> anyhow::Result<()> {
        execute!(stdout, cursor::MoveTo(0, 0))?;
        write!(
            stdout,
            "\x1b[1;96m~ niubash dino ~\x1b[0m  \x1b[1;93mHI {:<6}score {:<6}\x1b[0m\x1b[K\n\n",
            if self.best > 0 {
                self.best.to_string()
            } else {
                "-".into()
            },
            if self.phase == Phase::Over {
                format!("{} !", self.score)
            } else {
                self.score.to_string()
            }
        )?;

        // Sky, ground, and dirt: one string per row, then one write.
        let mut rows = vec![String::new(); GROUND_Y + 2];
        let dino_rows: &[&str] = if self.ducking && self.jump_y == 0 {
            DINO_DUCK
        } else if self.jump_y > 0 {
            DINO_STAND
        } else if self.run_frame {
            DINO_RUN_A
        } else {
            DINO_RUN_B
        };
        let dino_top = GROUND_Y - dino_rows.len() - self.jump_y;
        for (i, line) in dino_rows.iter().enumerate() {
            let row = &mut rows[dino_top + i];
            for (col, ch) in line.chars().enumerate() {
                if ch != ' ' {
                    row.push_str(&format!("\x1b[1;92m{}\x1b[0m", pad_to(row, DINO_X + col)));
                    row.push(ch);
                }
            }
        }
        for o in &self.obstacles {
            let (x, y, w, h) = obstacle_box(o);
            let body = match o.kind {
                Kind::Cactus { .. } => {
                    let mut c = String::new();
                    for _ in 0..w {
                        c.push_str("^| ");
                    }
                    vec![c.clone(), c.replace('^', "|"), c.replace('^', "|")]
                }
                Kind::Bird => vec!["==< ".to_string()],
            };
            for (i, line) in body.iter().take(h).enumerate() {
                let row = &mut rows[y + i];
                row.push_str(&format!("\x1b[1;91m{}\x1b[0m", pad_to(row, x)));
                row.push_str(line);
            }
        }

        for (i, row) in rows.iter().take(GROUND_Y + 1).enumerate() {
            let mut line = row.clone();
            if i == GROUND_Y {
                // The ground scrolls with the world.
                let shift = self.travel % 4;
                line.push_str(&"\x1b[90m".to_string());
                for c in 0..WIDTH {
                    line.push(if (c + shift) % 4 == 0 { '▔' } else { '_' });
                }
                line.push_str("\x1b[0m");
            }
            writeln!(stdout, " {line}\x1b[K")?;
        }
        // Scrolling dirt specks below the ground sell the speed.
        let shift = self.travel % 6;
        write!(stdout, " ")?;
        for c in 0..WIDTH {
            if (c + shift) % 6 == 0 {
                write!(stdout, "\x1b[90m·\x1b[0m")?;
            } else {
                write!(stdout, " ")?;
            }
        }
        writeln!(stdout, "\x1b[K")?;

        if self.phase == Phase::Over {
            writeln!(
                stdout,
                "\n  \x1b[1;91mCRASHED at {}!\x1b[0m \x1b[90mr restarts, q quits\x1b[0m\x1b[K",
                self.score
            )?;
        } else {
            writeln!(
                stdout,
                "\n  \x1b[90mspace/↑ jump   ↓ duck   q quit\x1b[0m\x1b[K"
            )?;
        }
        stdout.flush()?;
        Ok(())
    }
}

/// Spaces needed after `row` (ANSI included) so the next glyph lands in
/// column `target`; terminal cells only — the game field is ASCII-wide.
fn pad_to(row: &str, target: usize) -> String {
    let visible = crate::interactive_menu::display_width(row);
    " ".repeat(target.saturating_sub(visible))
}

fn rand_n(n: usize) -> usize {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;
    nanos % n.max(1)
}

pub(crate) fn run() -> anyhow::Result<i32> {
    if !crate::terminal::stdout_is_terminal() {
        return Ok(0);
    }
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let mut best = 0u32;
    let code = 'session: loop {
        let mut game = Game::new(best);
        let mut last_tick = Instant::now();
        'round: loop {
            game.draw(&mut stdout)?;
            while event::poll(Duration::ZERO)? {
                let Event::Key(KeyEvent {
                    code,
                    modifiers,
                    kind,
                    ..
                }) = event::read()?
                else {
                    continue;
                };
                // Windows reports Press/Repeat/Release; only Release means
                // "let go", which lifts a duck. Press and Repeat both act.
                let released = kind == KeyEventKind::Release;
                match code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break 'session 0,
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        break 'session 0
                    }
                    // A plain `break` here would only leave the `while`, so
                    // restarts target the labelled round loop.
                    KeyCode::Char('r') | KeyCode::Char('R') if game.phase == Phase::Over => {
                        break 'round;
                    }
                    KeyCode::Enter if game.phase == Phase::Over => break 'round,
                    KeyCode::Char(' ') | KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                        if game.phase == Phase::Over {
                            break 'round;
                        }
                        if !released {
                            game.jump();
                        }
                    }
                    KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                        game.ducking = !released && game.jump_y == 0;
                    }
                    _ => {}
                }
            }
            if last_tick.elapsed() >= Duration::from_millis(game.tick_ms()) {
                game.tick();
                last_tick = Instant::now();
            } else {
                // Frame pacing: never spin the core between ticks.
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        // Only reachable via `break 'round`: carry the score into the next
        // round before a fresh Game::new.
        best = best.max(game.score);
    };

    execute!(stdout, cursor::Show)?;
    execute!(stdout, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(code)
}
