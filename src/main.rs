mod db;
mod models;
mod notifications;
mod timer;
mod xp;

use chrono::{Datelike, Local, NaiveDate};
use iced::alignment;
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::mouse_area;
use iced::widget::{button, column, container, row, rule, space, text};
use iced::{time, window, Center, Color, Element, Fill, Padding, Subscription, Task, Theme};
use rusqlite::Connection;
use std::time::Duration;

use models::{Session, SessionType, UserProfile, FOCUS_DURATION_SECS, SESSIONS_BEFORE_LONG_BREAK};
use timer::{Timer, TimerState};

fn main() -> iced::Result {
    let window_settings = window::Settings {
        size: iced::Size::new(320.0, 540.0),
        decorations: false,
        ..Default::default()
    };

    iced::application(App::default, update, view)
        .title("Ferris Focus")
        .theme(Theme::CatppuccinMocha)
        .subscription(subscription)
        .window(window_settings)
        .centered()
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Timer,
    Stats,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    Start,
    PauseResume,
    Skip,
    SwitchView(View),
    DismissLevelUp,
    Minimize,
    Close,
    WindowReady(window::Id),
    DragStart,
}

struct App {
    timer: Timer,
    profile: UserProfile,
    current_view: View,
    db: Option<Connection>,
    session_start_time: Option<String>,
    today_sessions: u32,
    total_sessions: u32,
    total_focus_secs: u32,
    weekly_data: Vec<(String, u32)>,
    level_up: Option<u32>,
    window_id: Option<window::Id>,
}

impl Default for App {
    fn default() -> Self {
        let db = db::init_db().ok();
        let profile = db
            .as_ref()
            .and_then(|c| db::get_profile(c).ok())
            .unwrap_or_default();
        let today = Local::now().format("%Y-%m-%d").to_string();
        let today_sessions = db
            .as_ref()
            .and_then(|c| db::get_today_session_count(c, &today).ok())
            .unwrap_or(0);
        let (total_sessions, total_focus_secs) = db
            .as_ref()
            .and_then(|c| db::get_total_stats(c).ok())
            .unwrap_or((0, 0));

        let week_start = (Local::now() - chrono::Duration::days(6))
            .format("%Y-%m-%d")
            .to_string();
        let weekly_data = db
            .as_ref()
            .and_then(|c| db::get_sessions_in_range(c, &week_start, &today).ok())
            .unwrap_or_default();

        App {
            timer: Timer::new(),
            profile,
            current_view: View::Timer,
            db,
            session_start_time: None,
            today_sessions,
            total_sessions,
            total_focus_secs,
            weekly_data,
            level_up: None,
            window_id: None,
        }
    }
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Tick => {
            let finished = app.timer.tick();
            if finished {
                on_session_complete(app);
            }
            Task::none()
        }
        Message::Start => {
            let session_type = if app.timer.is_finished() {
                app.timer.next_session_type()
            } else {
                SessionType::Focus
            };
            app.timer.start(session_type);
            app.session_start_time = Some(Local::now().format("%Y-%m-%dT%H:%M:%S").to_string());
            Task::none()
        }
        Message::PauseResume => {
            if app.timer.is_running() {
                app.timer.pause();
            } else if app.timer.is_paused() {
                app.timer.resume();
            }
            Task::none()
        }
        Message::Skip => {
            if app.timer.is_running() || app.timer.is_paused() {
                app.timer.reset();
                app.session_start_time = None;
            }
            Task::none()
        }
        Message::SwitchView(v) => {
            app.current_view = v;
            if v == View::Stats {
                refresh_stats(app);
            }
            Task::none()
        }
        Message::DismissLevelUp => {
            app.level_up = None;
            Task::none()
        }
        Message::WindowReady(id) => {
            app.window_id = Some(id);
            Task::none()
        }
        Message::DragStart => {
            if let Some(id) = app.window_id {
                window::drag(id)
            } else {
                Task::none()
            }
        }
        Message::Minimize => {
            if let Some(id) = app.window_id {
                window::minimize(id, true)
            } else {
                Task::none()
            }
        }
        Message::Close => {
            if let Some(id) = app.window_id {
                window::close(id)
            } else {
                Task::none()
            }
        }
    }
}

fn on_session_complete(app: &mut App) {
    let session_type = app
        .timer
        .current_session_type()
        .unwrap_or(SessionType::Focus);
    let now = Local::now();
    let today: NaiveDate = now.date_naive();
    let completed_at = now.format("%Y-%m-%dT%H:%M:%S").to_string();

    let mut xp_earned = None;
    let old_level = app.profile.level;

    if session_type == SessionType::Focus {
        let new_streak = xp::update_streak(
            app.profile.last_session_date,
            today,
            app.profile.current_streak,
        );
        app.profile.current_streak = new_streak;
        if new_streak > app.profile.longest_streak {
            app.profile.longest_streak = new_streak;
        }
        app.profile.last_session_date = Some(today);

        let xp = xp::calculate_xp(app.profile.current_streak);
        app.profile.total_xp += xp;
        app.profile.level = xp::calculate_level(app.profile.total_xp);
        xp_earned = Some(xp);

        if app.profile.level > old_level {
            let new_stage = xp::ferris_stage(app.profile.level);
            notifications::notify_level_up(app.profile.level, new_stage);
            app.level_up = Some(app.profile.level);
        }

        app.today_sessions += 1;
        app.total_sessions += 1;
        app.total_focus_secs += FOCUS_DURATION_SECS;
    }

    if let Some(conn) = &app.db {
        let session = Session {
            id: None,
            started_at: app.session_start_time.clone().unwrap_or_default(),
            completed_at: Some(completed_at),
            duration_secs: app.timer.total_duration_secs(),
            session_type,
            completed: true,
        };
        let _ = db::save_session(conn, &session);
        let _ = db::update_profile(conn, &app.profile);
    }

    app.session_start_time = None;
    notifications::notify_session_complete(session_type, xp_earned);
}

fn refresh_stats(app: &mut App) {
    if let Some(conn) = &app.db {
        let today = Local::now().format("%Y-%m-%d").to_string();
        app.today_sessions = db::get_today_session_count(conn, &today).unwrap_or(0);
        let (total, secs) = db::get_total_stats(conn).unwrap_or((0, 0));
        app.total_sessions = total;
        app.total_focus_secs = secs;

        let week_start = (Local::now() - chrono::Duration::days(6))
            .format("%Y-%m-%d")
            .to_string();
        app.weekly_data = db::get_sessions_in_range(conn, &week_start, &today).unwrap_or_default();

        if let Ok(p) = db::get_profile(conn) {
            app.profile = p;
        }
    }
}

fn subscription(app: &App) -> Subscription<Message> {
    let timer_sub = if app.timer.is_running() {
        time::every(Duration::from_secs(1)).map(|_| Message::Tick)
    } else {
        Subscription::none()
    };

    let window_sub = if app.window_id.is_none() {
        window::open_events().map(Message::WindowReady)
    } else {
        Subscription::none()
    };

    Subscription::batch(vec![timer_sub, window_sub])
}

fn view_titlebar(_app: &App) -> Element<'_, Message> {
    let drag_area = mouse_area(
        space::horizontal()
            .width(iced::Length::Fill)
            .height(iced::Length::Fill),
    )
    .on_press(Message::DragStart);

    let minimize = mouse_area(text("─").size(20)).on_press(Message::Minimize);

    let close = mouse_area(text("✕").size(18)).on_press(Message::Close);

    row![
        drag_area,
        minimize,
        space::horizontal().width(iced::Length::Fixed(12.0)),
        close,
    ]
    .align_y(Center)
    .padding(Padding::from([0u16, 8]))
    .height(40)
    .into()
}

fn view(app: &App) -> Element<'_, Message> {
    let titlebar = view_titlebar(app);

    let content: Element<Message> = match app.current_view {
        View::Timer => view_timer(app),
        View::Stats => view_stats(app),
    };

    let nav = view_nav(app);

    let layout = column![titlebar, content, space::vertical().height(8), nav,]
        .padding(Padding::from([0u16, 24]))
        .spacing(0)
        .width(Fill)
        .height(Fill);

    let main_view = container(layout).width(Fill).height(Fill);

    if let Some(level) = app.level_up {
        let stage = xp::ferris_stage(level);
        let prev_stage = xp::ferris_stage(level.saturating_sub(1));

        let modal = column![
            text("🎉 Level Up! 🎉").size(28),
            space::vertical().height(20),
            row![
                text(format!("{}", prev_stage.emoji())).size(48),
                text(" → ").size(32),
                text(format!("{}", stage.emoji())).size(48),
            ],
            space::vertical().height(12),
            text(format!("Level {}", level)).size(24),
            space::vertical().height(8),
            text(format!("{}", stage.label())).size(18),
            space::vertical().height(24),
            button(text("Continue").size(16))
                .on_press(Message::DismissLevelUp)
                .padding([12, 24])
                .style(button::primary),
        ]
        .align_x(Center)
        .spacing(0)
        .padding(32);

        let modal_container = container(modal)
            .width(iced::Length::Fill)
            .center_x(iced::Length::Fill)
            .center_y(iced::Length::Fill);

        column![main_view, modal_container].into()
    } else {
        main_view.into()
    }
}

fn view_timer(app: &App) -> Element<'_, Message> {
    let stage = xp::ferris_stage(app.profile.level);
    let header = row![
        text(format!("{} Ferris Focus", stage.emoji())).size(20),
        space::horizontal(),
        text(format!("Lv. {}", app.profile.level)).size(18),
    ]
    .width(Fill);

    let timer_canvas = Canvas::new(TimerWidget {
        progress: app.timer.progress(),
        remaining: app.timer.remaining_display(),
        session_label: app
            .timer
            .current_session_type()
            .map(|t| t.label())
            .unwrap_or("READY"),
        is_idle: matches!(app.timer.state, TimerState::Idle),
        is_finished: app.timer.is_finished(),
    })
    .width(220)
    .height(220);

    let timer_row = row![space::horizontal(), timer_canvas, space::horizontal()];

    let controls = view_controls(app);

    let streak_xp = row![
        text(format!("🔥 Streak: {} days", app.profile.current_streak)).size(14),
        space::horizontal(),
        text(format!("⭐ {} XP", app.profile.total_xp)).size(14),
    ]
    .width(Fill);

    let level_progress = xp::level_progress(app.profile.total_xp);
    let xp_bar = view_progress_bar(level_progress, 12.0);

    let session_count = app.timer.focus_sessions_completed % SESSIONS_BEFORE_LONG_BREAK;
    let session_info = text(format!(
        "Session: {}/{} until long break",
        session_count, SESSIONS_BEFORE_LONG_BREAK
    ))
    .size(12);

    column![
        header,
        space::vertical().height(20),
        timer_row,
        space::vertical().height(20),
        controls,
        space::vertical().height(20),
        streak_xp,
        space::vertical().height(6),
        xp_bar,
        space::vertical().height(8),
        session_info,
    ]
    .spacing(0)
    .width(Fill)
    .into()
}

fn view_controls(app: &App) -> Element<'_, Message> {
    let is_idle = matches!(app.timer.state, TimerState::Idle);
    let is_finished = app.timer.is_finished();

    if is_idle {
        row![
            space::horizontal(),
            button(text("▶  Start Focus").size(16).align_x(Center))
                .on_press(Message::Start)
                .padding([10, 28])
                .style(button::primary),
            space::horizontal(),
        ]
        .width(Fill)
        .into()
    } else if is_finished {
        let next_label = match app.timer.next_session_type() {
            SessionType::Focus => "▶  Start Focus",
            SessionType::ShortBreak => "☕  Short Break",
            SessionType::LongBreak => "🎉  Long Break",
        };
        row![
            space::horizontal(),
            button(text(next_label).size(16).align_x(Center))
                .on_press(Message::Start)
                .padding([10, 28])
                .style(button::primary),
            space::horizontal(),
        ]
        .width(Fill)
        .into()
    } else {
        let pause_label = if app.timer.is_paused() {
            "▶  Resume"
        } else {
            "⏸  Pause"
        };
        row![
            space::horizontal(),
            button(text(pause_label).size(14).align_x(Center))
                .on_press(Message::PauseResume)
                .padding([8, 20])
                .style(button::primary),
            button(text("⏭  Skip").size(14).align_x(Center))
                .on_press(Message::Skip)
                .padding([8, 20])
                .style(button::secondary),
            space::horizontal(),
        ]
        .spacing(12)
        .width(Fill)
        .into()
    }
}

fn view_progress_bar(progress: f32, height: f32) -> Element<'static, Message> {
    Canvas::new(ProgressBarWidget {
        progress: progress.clamp(0.0, 1.0),
    })
    .width(Fill)
    .height(height)
    .into()
}

fn view_stats(app: &App) -> Element<'_, Message> {
    let stage = xp::ferris_stage(app.profile.level);

    let title = text("📊 Stats & Progress").size(22);

    let ferris_info = row![
        text(format!("{}", stage.emoji())).size(48),
        column![
            text(stage.label()).size(18),
            text(format!("Level {}", app.profile.level)).size(14),
        ]
        .spacing(4),
    ]
    .spacing(16)
    .align_y(Center);

    let today_label = text(format!("Today: {} focus sessions", app.today_sessions)).size(14);

    let total_hours = app.total_focus_secs / 3600;
    let total_mins = (app.total_focus_secs % 3600) / 60;
    let total_label = text(format!(
        "All time: {} sessions • {}h {}m focused",
        app.total_sessions, total_hours, total_mins
    ))
    .size(14);

    let streak_label = text(format!(
        "🔥 Current streak: {} days  •  Best: {} days",
        app.profile.current_streak, app.profile.longest_streak
    ))
    .size(14);

    let xp_label = text(format!("⭐ Total XP: {}", app.profile.total_xp)).size(14);

    let heatmap_title = text("Last 7 Days").size(16);
    let heatmap = view_weekly_heatmap(app);

    column![
        title,
        space::vertical().height(16),
        ferris_info,
        space::vertical().height(16),
        rule::horizontal(1),
        space::vertical().height(12),
        today_label,
        total_label,
        space::vertical().height(8),
        streak_label,
        xp_label,
        space::vertical().height(16),
        rule::horizontal(1),
        space::vertical().height(12),
        heatmap_title,
        space::vertical().height(8),
        heatmap,
    ]
    .spacing(2)
    .width(Fill)
    .into()
}

fn view_weekly_heatmap(app: &App) -> Element<'_, Message> {
    let today = Local::now().date_naive();
    let days: Vec<NaiveDate> = (0..7)
        .rev()
        .map(|i| today - chrono::Duration::days(i))
        .collect();

    let day_labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    let boxes: Vec<Element<Message>> = days
        .iter()
        .map(|date| {
            let date_str = date.format("%Y-%m-%d").to_string();
            let count = app
                .weekly_data
                .iter()
                .find(|(d, _)| d == &date_str)
                .map(|(_, c)| *c)
                .unwrap_or(0);

            let weekday_idx = date.weekday().num_days_from_monday() as usize;
            let label = day_labels[weekday_idx];

            column![
                Canvas::new(HeatmapCell { count }).width(40).height(40),
                text(label).size(11),
            ]
            .spacing(4)
            .align_x(Center)
            .into()
        })
        .collect();

    let mut heatmap_row = row![].spacing(6);
    for b in boxes {
        heatmap_row = heatmap_row.push(b);
    }

    heatmap_row.into()
}

fn view_nav(app: &App) -> Element<'_, Message> {
    let timer_style = if app.current_view == View::Timer {
        button::primary
    } else {
        button::secondary
    };
    let stats_style = if app.current_view == View::Stats {
        button::primary
    } else {
        button::secondary
    };

    row![
        button(text("⏱  Timer").size(14).align_x(Center))
            .on_press(Message::SwitchView(View::Timer))
            .padding([8, 20])
            .width(Fill)
            .style(timer_style),
        button(text("📊  Stats").size(14).align_x(Center))
            .on_press(Message::SwitchView(View::Stats))
            .padding([8, 20])
            .width(Fill)
            .style(stats_style),
    ]
    .spacing(8)
    .width(Fill)
    .into()
}

// -- canvas widgets --

struct TimerWidget<'a> {
    progress: f32,
    remaining: (u32, u32),
    session_label: &'a str,
    is_idle: bool,
    is_finished: bool,
}

impl<'a> canvas::Program<Message> for TimerWidget<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let center = frame.center();
        let radius = bounds.width.min(bounds.height) / 2.0 - 10.0;

        let palette = theme.palette();

        let bg_circle = Path::circle(center, radius);
        frame.stroke(
            &bg_circle,
            Stroke::default().with_width(8.0).with_color(Color {
                a: 0.15,
                ..palette.text
            }),
        );

        if !self.is_idle {
            let progress_color = if self.is_finished {
                Color::from_rgb(0.4, 0.9, 0.4)
            } else {
                palette.primary
            };

            let start_angle = -std::f32::consts::FRAC_PI_2;
            let segments = 60;
            let steps = (segments as f32 * self.progress) as usize;
            if steps > 0 {
                let mut builder = canvas::path::Builder::new();
                for i in 0..=steps {
                    let angle = start_angle + (i as f32 / segments as f32) * std::f32::consts::TAU;
                    let x = center.x + radius * angle.cos();
                    let y = center.y + radius * angle.sin();
                    if i == 0 {
                        builder.move_to(iced::Point::new(x, y));
                    } else {
                        builder.line_to(iced::Point::new(x, y));
                    }
                }
                let arc_path = builder.build();
                frame.stroke(
                    &arc_path,
                    Stroke::default().with_width(8.0).with_color(progress_color),
                );
            }
        }

        let time_str = if self.is_idle {
            "25:00".to_string()
        } else if self.is_finished {
            "Done!".to_string()
        } else {
            format!("{:02}:{:02}", self.remaining.0, self.remaining.1)
        };

        frame.fill_text(canvas::Text {
            content: time_str,
            position: iced::Point::new(center.x, center.y - 10.0),
            color: palette.text,
            size: iced::Pixels(42.0),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: alignment::Vertical::Center,
            ..canvas::Text::default()
        });

        frame.fill_text(canvas::Text {
            content: self.session_label.to_string(),
            position: iced::Point::new(center.x, center.y + 25.0),
            color: Color {
                a: 0.6,
                ..palette.text
            },
            size: iced::Pixels(14.0),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: alignment::Vertical::Center,
            ..canvas::Text::default()
        });

        vec![frame.into_geometry()]
    }
}

struct ProgressBarWidget {
    progress: f32,
}

impl canvas::Program<Message> for ProgressBarWidget {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = theme.palette();

        let bar_height = bounds.height;

        let bg = Path::rectangle(
            iced::Point::ORIGIN,
            iced::Size::new(bounds.width, bar_height),
        );
        frame.fill(
            &bg,
            Color {
                a: 0.15,
                ..palette.text
            },
        );

        let fill_width = bounds.width * self.progress;
        if fill_width > 0.0 {
            let fill =
                Path::rectangle(iced::Point::ORIGIN, iced::Size::new(fill_width, bar_height));
            frame.fill(&fill, palette.primary);
        }

        vec![frame.into_geometry()]
    }
}

struct HeatmapCell {
    count: u32,
}

impl canvas::Program<Message> for HeatmapCell {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = theme.palette();

        let intensity = match self.count {
            0 => 0.08,
            1 => 0.3,
            2 => 0.5,
            3 => 0.7,
            _ => 0.9,
        };

        let color = Color {
            a: intensity,
            ..palette.primary
        };

        let rect = Path::rectangle(
            iced::Point::new(2.0, 2.0),
            iced::Size::new(bounds.width - 4.0, bounds.height - 4.0),
        );
        frame.fill(&rect, color);

        if self.count > 0 {
            frame.fill_text(canvas::Text {
                content: self.count.to_string(),
                position: iced::Point::new(bounds.width / 2.0, bounds.height / 2.0),
                color: palette.text,
                size: iced::Pixels(13.0),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: alignment::Vertical::Center,
                ..canvas::Text::default()
            });
        }

        vec![frame.into_geometry()]
    }
}
