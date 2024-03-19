use crossterm::event::MouseEvent;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};

use super::Component;

pub fn centered_box_sized(rect: Rect, width: u16, height: u16) -> Rect {
    let x_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(rect);
    let y_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(x_layout[1]);
    y_layout[1]
}

pub fn centered_box_percentage(rect: Rect, x_percent: u16, y_percent: u16) -> Rect {
    let x_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - x_percent) / 2),
            Constraint::Percentage(x_percent),
            Constraint::Percentage((100 - x_percent) / 2),
        ])
        .split(rect);
    let y_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - y_percent) / 2),
            Constraint::Percentage(x_percent),
            Constraint::Percentage((100 - y_percent) / 2),
        ])
        .split(x_layout[1]);
    y_layout[1]
}

pub fn mouse_contains<'a, T: Component>(
    mouse: &MouseEvent,
    component: Option<&'a mut T>,
) -> (bool, &'a mut T) {
    let com = component.unwrap();
    (
        com.mouse_area()
            .contains(Position::new(mouse.column, mouse.row)),
        com,
    )
}
