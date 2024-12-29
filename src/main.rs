use ::input::event::keyboard::KeyState;
use ::input::event::keyboard::KeyboardEventTrait;
use ::input::Event;
use anyhow::anyhow;
use anyhow::Result;
use client::Client;
use color::Color;
use glam::I16Vec2;
use glam::U16Vec2;
use input::Key;
use std::f32::consts::FRAC_PI_4;
use std::f32::consts::PI;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

mod client;
mod color;
mod input;

/// Height of a paddle.
const PADDLE_HEIGHT_FRACTION: f32 = 1.0 / 7.0;
/// Width of a paddle.
const PADDLE_WIDTH: i16 = 47;
/// Gap to left or right.
const PADDLE_GAP_FRACTION: f32 = 1.0 / 9.0;
/// Size of the ball.
const BALL_SIZE: i16 = 58;
const BALL_SPEED: f32 = 30.0;
const PADDLE_SPEED: i16 = 17;

// 75 degrees
const MAX_REFLECT_ANGLE: f32 = FRAC_PI_4;

const FRAME_TIME: Duration = Duration::from_millis(1000 / 30);

const OBJECT_COLOR: Color = Color {
    r: 0xff,
    g: 0,
    b: 0xff,
    a: 0xff,
};

const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 0xff,
};
const BORDER_WIDTH: i16 = 6;

#[derive(Clone, Copy, Debug)]
enum PlayerDir {
    Up,
    Down,
    Neutral,
    // both keys are pressed - behaves like neutral, but improved user experience
    Both,
}

impl PlayerDir {
    pub const fn press_up(self) -> Self {
        match self {
            PlayerDir::Up | PlayerDir::Neutral => PlayerDir::Up,
            PlayerDir::Down | PlayerDir::Both => PlayerDir::Both,
        }
    }
    pub const fn press_down(self) -> Self {
        match self {
            PlayerDir::Down | PlayerDir::Neutral => PlayerDir::Down,
            PlayerDir::Up | PlayerDir::Both => PlayerDir::Both,
        }
    }
    pub const fn release_up(self) -> Self {
        match self {
            PlayerDir::Up | PlayerDir::Neutral => PlayerDir::Neutral,
            PlayerDir::Down | PlayerDir::Both => PlayerDir::Down,
        }
    }
    pub const fn release_down(self) -> Self {
        match self {
            PlayerDir::Down | PlayerDir::Neutral => PlayerDir::Neutral,
            PlayerDir::Up | PlayerDir::Both => PlayerDir::Up,
        }
    }
}

struct GameState {
    /// Game grid size.
    size: I16Vec2,
    /// Position of the center of the ball.
    ball: I16Vec2,
    /// Travelling angle of the ball.
    ball_angle: f32,
    /// Whether ball is moving. Ball stops moving after a player scores.
    ball_is_moving: bool,
    /// Player 1 position (top left).
    player1: I16Vec2,
    /// Player 2 position (top left).
    player2: I16Vec2,
    player1_dir: PlayerDir,
    player2_dir: PlayerDir,

    paddle_height: i16,
}

impl GameState {
    pub fn new(size: I16Vec2) -> SharedGameState {
        let paddle_height = (PADDLE_HEIGHT_FRACTION * size.y as f32).floor() as i16;
        let paddle_y = (size.y - paddle_height) / 2;
        Arc::new(RwLock::new(Self {
            size,
            ball: size / 2,
            ball_angle: 0.0,
            player1: I16Vec2::new(
                (PADDLE_GAP_FRACTION * size.x as f32).floor() as i16,
                paddle_y,
            ),
            player2: I16Vec2::new(
                size.x - (PADDLE_GAP_FRACTION * size.x as f32).floor() as i16 - PADDLE_WIDTH,
                paddle_y,
            ),
            ball_is_moving: false,
            player1_dir: PlayerDir::Neutral,
            player2_dir: PlayerDir::Neutral,
            paddle_height,
        }))
    }

    pub fn update(&mut self) {
        self.player1 += I16Vec2::from(match self.player1_dir {
            PlayerDir::Up => (0i16, -PADDLE_SPEED),
            PlayerDir::Down => (0, PADDLE_SPEED),
            PlayerDir::Neutral | PlayerDir::Both => (0, 0),
        });

        self.player2 += I16Vec2::from(match self.player2_dir {
            PlayerDir::Up => (0i16, -PADDLE_SPEED),
            PlayerDir::Down => (0, PADDLE_SPEED),
            PlayerDir::Neutral | PlayerDir::Both => (0, 0),
        });

        self.player1.y = self.player1.y.clamp(0, self.size.y - self.paddle_height);
        self.player2.y = self.player2.y.clamp(0, self.size.y - self.paddle_height);

        if self.ball_is_moving {
            self.ball += I16Vec2 {
                x: (self.ball_angle.cos() * BALL_SPEED) as i16,
                y: (self.ball_angle.sin() * BALL_SPEED) as i16,
            };
        }

        if self.ball.y < 0 || self.ball.y > self.size.y {
            self.ball_angle = (PI * 2.0) - self.ball_angle;
            self.ball.y = self.ball.y.clamp(0, self.size.y);
        }

        // player 1 ball collision
        if self.ball.x > self.player1.x
            && self.ball.x < self.player1.x + PADDLE_WIDTH
            && self.ball.y > self.player1.y
            && self.ball.y < self.player1.y + self.paddle_height
            && self.ball_angle.cos() < 0.0
        {
            // [-1, 1] normalized position of the collision relative to the center
            let collision_pos = -((self.paddle_height / 2) - (self.ball.y - self.player1.y)) as f32
                / (self.paddle_height / 2) as f32;
            let collision_angle = MAX_REFLECT_ANGLE * collision_pos;
            self.ball_angle = collision_angle;
        }

        // player 2 ball collision
        if self.ball.x > self.player2.x
            && self.ball.x < self.player2.x + PADDLE_WIDTH
            && self.ball.y > self.player2.y
            && self.ball.y < self.player2.y + self.paddle_height
            && self.ball_angle.cos() > 0.0
        {
            // [-1, 1] normalized position of the collision relative to the center
            let collision_pos = ((self.paddle_height / 2) - (self.ball.y - self.player2.y)) as f32
                / (self.paddle_height / 2) as f32;
            let collision_angle = MAX_REFLECT_ANGLE * collision_pos - PI;

            self.ball_angle = collision_angle;
        }

        // player 1 scores
        if self.ball.x > self.size.x {
            self.ball_is_moving = false;
            self.ball = self.size / 2 + I16Vec2::new(self.size.x / 3, 0);
            self.ball_angle = PI;
        }
        // player 2 scores
        else if self.ball.x < 0 {
            self.ball_is_moving = false;
            self.ball = self.size / 2 - I16Vec2::new(self.size.x / 3, 0);
            self.ball_angle = 0.0;
        }
    }

    pub fn handle_press(&mut self, key: Key, state: KeyState) {
        match (key, state) {
            (Key::W, KeyState::Pressed) => self.player1_dir = self.player1_dir.press_up(),
            (Key::S, KeyState::Pressed) => self.player1_dir = self.player1_dir.press_down(),
            (Key::W, KeyState::Released) => self.player1_dir = self.player1_dir.release_up(),
            (Key::S, KeyState::Released) => self.player1_dir = self.player1_dir.release_down(),
            (Key::Up, KeyState::Pressed) => self.player2_dir = self.player2_dir.press_up(),
            (Key::Down, KeyState::Pressed) => self.player2_dir = self.player2_dir.press_down(),
            (Key::Up, KeyState::Released) => self.player2_dir = self.player2_dir.release_up(),
            (Key::Down, KeyState::Released) => self.player2_dir = self.player2_dir.release_down(),
            (Key::Space, _) => {}
        }
        // any key pressed -> start ball to move
        if state == KeyState::Pressed {
            self.ball_is_moving = true;
        }
    }
}

type SharedGameState = Arc<RwLock<GameState>>;

fn draw_ball(game: SharedGameState, server: String) -> Result<()> {
    let mut client = Client::new(TcpStream::connect(server)?, false, true);

    // let mut random = random::default(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_micros() as u64);

    loop {
        let game = game.read().unwrap();
        let ball_pos = game.ball;
        drop(game);

        // let color: Color = random.read();
        let color = OBJECT_COLOR;
        let upper_left = ball_pos - BALL_SIZE / 2;
        for x in 0..BALL_SIZE {
            for y in 0..BALL_SIZE {
                let pos = upper_left + I16Vec2::new(x, y);

                let color = if x < BORDER_WIDTH
                    || x > BALL_SIZE - BORDER_WIDTH
                    || y < BORDER_WIDTH
                    || y > BALL_SIZE - BORDER_WIDTH
                {
                    BLACK
                } else {
                    color
                };

                client.write_pixel(pos.x as u16, pos.y as u16, color)?;
            }
        }
    }
}
fn draw_players(game: SharedGameState, server: String) -> Result<()> {
    let mut client = Client::new(TcpStream::connect(server)?, false, true);

    // let mut random = random::default(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_micros() as u64);

    let paddle_height =
        (PADDLE_HEIGHT_FRACTION * game.read().unwrap().size.y as f32).floor() as i16;

    loop {
        let game = game.read().unwrap();
        let player1_pos = game.player1;
        let player2_pos = game.player2;
        drop(game);

        // let color: Color = random.read();
        let color = OBJECT_COLOR;
        for x in 0..PADDLE_WIDTH {
            for y in 0..paddle_height {
                let pos1 = player1_pos + I16Vec2::new(x, y);
                let pos2 = player2_pos + I16Vec2::new(x, y);
                let color = if x < BORDER_WIDTH
                    || x > PADDLE_WIDTH - BORDER_WIDTH
                    || y < BORDER_WIDTH
                    || y > paddle_height - BORDER_WIDTH
                {
                    BLACK
                } else {
                    color
                };
                client.write_pixel(pos1.x as u16, pos1.y as u16, color)?;
                client.write_pixel(pos2.x as u16, pos2.y as u16, color)?;
            }
        }
    }
}

fn handle_user_input(game: SharedGameState) -> Result<()> {
    let mut input = input::Interface::new();
    loop {
        input.dispatch().unwrap();
        for event in (&mut input).filter_map(|ev| match ev {
            Event::Keyboard(kb) => Some(kb),
            _ => None,
        }) {
            let code = event.key();
            let key = input::Key::try_from(code);
            if let Ok(key) = key {
                let state = event.key_state();
                game.write().unwrap().handle_press(key, state);
            }
        }
    }
}

fn main() -> Result<()> {
    let server = std::env::args()
        .nth(1)
        .ok_or(anyhow!("usage: pongxelflut [host:port]"))?;
    let mut size_client = Client::new(TcpStream::connect(&server)?, false, true);
    let size = U16Vec2::from(size_client.read_screen_size()?);
    let size = I16Vec2::new(size.x as i16, size.y as i16);

    let game = GameState::new(size);
    let game_for_ball = game.clone();
    let game_for_players = game.clone();
    let game_for_input = game.clone();
    let server2 = server.clone();
    let server3 = server.clone();
    std::thread::spawn(move || loop {
        let _ = draw_ball(game_for_ball.clone(), server2.clone());
    });
    std::thread::spawn(move || loop {
        let _ = draw_players(game_for_players.clone(), server3.clone());
    });
    std::thread::spawn(move || loop {
        let _ = handle_user_input(game_for_input.clone());
    });

    let mut last_update = Instant::now();
    let mut delta = Duration::ZERO;
    loop {
        let now = Instant::now();
        delta += now - last_update;

        if delta > FRAME_TIME {
            game.write().unwrap().update();
            delta -= FRAME_TIME;
        } else {
            sleep(FRAME_TIME / 2);
        }
        last_update = now;
    }
}
