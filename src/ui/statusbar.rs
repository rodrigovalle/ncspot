use std::sync::Arc;
use std::io::{self, Write};

use cursive::align::HAlign;
use cursive::event::{Event, EventResult, MouseButton, MouseEvent};
use cursive::theme::{ColorStyle, ColorType, PaletteColor};
use cursive::traits::View;
use cursive::vec::Vec2;
use cursive::Printer;
use unicode_width::UnicodeWidthStr;

use library::Library;
use queue::{Queue, RepeatSetting};
use spotify::{PlayerEvent, Spotify};

pub struct StatusBar {
    queue: Arc<Queue>,
    spotify: Arc<Spotify>,
    library: Arc<Library>,
    last_size: Vec2,
    use_nerdfont: bool,
}

impl StatusBar {
    pub fn new(queue: Arc<Queue>, library: Arc<Library>, use_nerdfont: bool) -> StatusBar {
        let spotify = queue.get_spotify();

        StatusBar {
            queue,
            spotify,
            library,
            last_size: Vec2::new(0, 0),
            use_nerdfont,
        }
    }
}

impl View for StatusBar {
    fn draw(&self, printer: &Printer<'_, '_>) {
        if printer.size.x == 0 {
            return;
        }

        let style_bar = ColorStyle::new(
            ColorType::Color(*printer.theme.palette.custom("statusbar_progress").unwrap()),
            ColorType::Palette(PaletteColor::Background),
        );
        let style_bar_bg = ColorStyle::new(
            ColorType::Color(
                *printer
                    .theme
                    .palette
                    .custom("statusbar_progress_bg")
                    .unwrap(),
            ),
            ColorType::Palette(PaletteColor::Background),
        );
        let style = ColorStyle::new(
            ColorType::Color(*printer.theme.palette.custom("statusbar").unwrap()),
            ColorType::Color(*printer.theme.palette.custom("statusbar_bg").unwrap()),
        );

        printer.print(
            (0, 0),
            &vec![' '; printer.size.x].into_iter().collect::<String>(),
        );
        printer.with_color(style, |printer| {
            printer.print(
                (0, 1),
                &vec![' '; printer.size.x].into_iter().collect::<String>(),
            );
        });

        let state_icon = if self.use_nerdfont {
            match self.spotify.get_current_status() {
                PlayerEvent::Playing => "\u{f909} ",
                PlayerEvent::Paused => "\u{f8e3} ",
                PlayerEvent::Stopped | PlayerEvent::FinishedTrack => "\u{f9da} ",
            }
        } else {
            match self.spotify.get_current_status() {
                PlayerEvent::Playing => "▶ ",
                PlayerEvent::Paused => "▮▮",
                PlayerEvent::Stopped | PlayerEvent::FinishedTrack => "◼ ",
            }
        }
        .to_string();

        printer.with_color(style, |printer| {
            printer.print((1, 1), &state_icon);
        });

        let repeat = if self.use_nerdfont {
            match self.queue.get_repeat() {
                RepeatSetting::None => "",
                RepeatSetting::RepeatPlaylist => "\u{f955} ",
                RepeatSetting::RepeatTrack => "\u{f957} ",
            }
        } else {
            match self.queue.get_repeat() {
                RepeatSetting::None => "",
                RepeatSetting::RepeatPlaylist => "[R] ",
                RepeatSetting::RepeatTrack => "[R1] ",
            }
        };

        let shuffle = if self.queue.get_shuffle() {
            if self.use_nerdfont {
                "\u{f99c} "
            } else {
                "[Z] "
            }
        } else {
            ""
        };

        printer.with_color(style_bar_bg, |printer| {
            printer.print((0, 0), &"┉".repeat(printer.size.x));
        });

        if let Some(t) = self.queue.get_current() {
            let elapsed = self.spotify.get_current_progress();
            let elapsed_ms = elapsed.as_secs() as u32 * 1000 + elapsed.subsec_millis();

            let formatted_elapsed = format!(
                "{:02}:{:02}",
                elapsed.as_secs() / 60,
                elapsed.as_secs() % 60
            );

            let saved = if self.library.is_saved_track(&t) {
                if self.use_nerdfont {
                    "\u{f62b} "
                } else {
                    "✓ "
                }
            } else {
                ""
            };

            let right = repeat.to_string()
                + shuffle
                + saved
                + &format!("{} / {} ", formatted_elapsed, t.duration_str());
            let offset = HAlign::Right.get_offset(right.width(), printer.size.x);

            printer.with_color(style, |printer| {
                printer.print((4, 1), &t.to_string());
                printer.print((offset, 1), &right);
            });

            printer.with_color(style_bar, |printer| {
                let duration_width = (((printer.size.x as u32) * elapsed_ms) / t.duration) as usize;
                printer.print((0, 0), &"━".repeat(duration_width + 1));
            });

            // update window title
            let state_string = match self.spotify.get_current_status() {
                PlayerEvent::Playing => "Playing",
                PlayerEvent::Paused => "Paused",
                PlayerEvent::Stopped | PlayerEvent::FinishedTrack => "",
            };
            print!("\x1B]2;[{}] {}\x07", state_string, &t.to_string());
            io::stdout().flush();

        } else {
            let right = repeat.to_string() + shuffle;
            let offset = HAlign::Right.get_offset(right.width(), printer.size.x);

            printer.with_color(style, |printer| {
                printer.print((offset, 1), &right);
            });

            // clear the window title
            print!("\x1b]2;ncspot\x07");
            io::stdout().flush();
        }
    }

    fn layout(&mut self, size: Vec2) {
        self.last_size = size;
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        Vec2::new(constraint.x, 2)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if let Event::Mouse {
            offset,
            position,
            event,
        } = event
        {
            let position = position - offset;

            if position.y == 0 {
                if event == MouseEvent::WheelUp {
                    self.spotify.seek_relative(-500);
                }

                if event == MouseEvent::WheelDown {
                    self.spotify.seek_relative(500);
                }

                if event == MouseEvent::Press(MouseButton::Left)
                    || event == MouseEvent::Hold(MouseButton::Left)
                {
                    if let Some(ref t) = self.queue.get_current() {
                        let f: f32 = position.x as f32 / self.last_size.x as f32;
                        let new = t.duration as f32 * f;
                        self.spotify.seek(new as u32);
                    }
                }
            } else if event == MouseEvent::Press(MouseButton::Left) {
                self.queue.toggleplayback();
            }

            EventResult::Consumed(None)
        } else {
            EventResult::Ignored
        }
    }
}
